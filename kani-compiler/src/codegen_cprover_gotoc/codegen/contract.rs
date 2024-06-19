// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::attributes::KaniAttributes;
use cbmc::goto_program::FunctionContract;
use cbmc::goto_program::{Lambda, Location};
use kani_metadata::AssignsContract;
use rustc_hir::def_id::DefId as InternalDefId;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::Local;
use stable_mir::CrateDef;
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    /// Given the `proof_for_contract` target `function_under_contract` and the reachable `items`,
    /// find or create the `AssignsContract` that needs to be enforced and attach it to the symbol
    /// for which it needs to be enforced.
    ///
    /// 1. Gets the `#[kanitool::inner_check = "..."]` target, then resolves exactly one instance
    ///    of it. Panics if there are more or less than one instance.
    /// 2. Expects that a `#[kanitool::modifies(...)]` is placed on the `inner_check` function,
    ///    turns it into a CBMC contract and attaches it to the symbol for the previously resolved
    ///    instance.
    /// 3. Returns the mangled name of the symbol it attached the contract to.
    /// 4. Resolves the `#[kanitool::checked_with = "..."]` target from `function_under_contract`
    ///    which has `static mut REENTRY : bool` declared inside.
    /// 5. Returns the full path to this constant that `--nondet-static-exclude` expects which is
    ///    comprised of the file path that `checked_with` is located in, the name of the
    ///    `checked_with` function and the name of the constant (`REENTRY`).
    pub fn handle_check_contract(
        &mut self,
        function_under_contract: InternalDefId,
        items: &[MonoItem],
    ) -> AssignsContract {
        let tcx = self.tcx;
        let function_under_contract_attrs = KaniAttributes::for_item(tcx, function_under_contract);

        let recursion_wrapper_id =
            function_under_contract_attrs.checked_with_id().unwrap().unwrap();
        let expected_name = format!("{}::REENTRY", tcx.item_name(recursion_wrapper_id));
        let mut recursion_tracker = items.iter().filter_map(|i| match i {
            MonoItem::Static(recursion_tracker)
                if (*recursion_tracker).name().contains(expected_name.as_str()) =>
            {
                Some(*recursion_tracker)
            }
            _ => None,
        });

        let recursion_tracker_def = recursion_tracker
            .next()
            .expect("There should be at least one recursion tracker (REENTRY) in scope");
        assert!(
            recursion_tracker.next().is_none(),
            "Only one recursion tracker (REENTRY) may be in scope"
        );

        let span_of_recursion_wrapper = tcx.def_span(recursion_wrapper_id);
        let location_of_recursion_wrapper = self.codegen_span(&span_of_recursion_wrapper);
        // The name and location for the recursion tracker should match the exact information added
        // to the symbol table, otherwise our contract instrumentation will silently failed.
        // This happens because Kani relies on `--nondet-static-exclude` from CBMC to properly
        // handle this tracker. CBMC silently fails if there is no match in the symbol table
        // that correspond to the argument of this flag.
        // More details at https://github.com/model-checking/kani/pull/3045.
        let full_recursion_tracker_name = format!(
            "{}:{}",
            location_of_recursion_wrapper
                .filename()
                .expect("recursion location wrapper should have a file name"),
            // We must use the pretty name of the tracker instead of the mangled name.
            // This restrictions comes from `--nondet-static-exclude` in CBMC.
            // Mode details at https://github.com/diffblue/cbmc/issues/8225.
            recursion_tracker_def.name(),
        );

        let wrapped_fn = function_under_contract_attrs.inner_check().unwrap().unwrap();
        let mut instance_under_contract = items.iter().filter_map(|i| match i {
            MonoItem::Fn(instance @ Instance { def, .. })
                if wrapped_fn == rustc_internal::internal(tcx, def.def_id()) =>
            {
                Some(*instance)
            }
            _ => None,
        });
        let instance_of_check = instance_under_contract.next().unwrap();
        assert!(
            instance_under_contract.next().is_none(),
            "Only one instance of a checked function may be in scope"
        );
        let attrs_of_wrapped_fn = KaniAttributes::for_item(tcx, wrapped_fn);
        let assigns_contract = attrs_of_wrapped_fn.modifies_contract().unwrap_or_else(|| {
            debug!(?instance_of_check, "had no assigns contract specified");
            vec![]
        });
        self.attach_modifies_contract(instance_of_check, assigns_contract);
        let wrapper_name = self.symbol_name_stable(instance_of_check);

        AssignsContract {
            recursion_tracker: full_recursion_tracker_name,
            contracted_function_name: wrapper_name,
        }
    }

    /// Convert the Kani level contract into a CBMC level contract by creating a
    /// CBMC lambda.
    fn codegen_modifies_contract(
        &mut self,
        modified_places: Vec<Local>,
        loc: Location,
    ) -> FunctionContract {
        let goto_annotated_fn_name = self.current_fn().name();
        let goto_annotated_fn_typ = self
            .symbol_table
            .lookup(&goto_annotated_fn_name)
            .unwrap_or_else(|| panic!("Function '{goto_annotated_fn_name}' is not declared"))
            .typ
            .clone();

        let shadow_memory_symbol = {
            let attr_id = self
                .tcx
                .all_diagnostic_items(())
                .name_to_id
                .get(&rustc_span::symbol::Symbol::intern("KaniShadowMemory"))
                .unwrap();
            let shadow_memory_table = self
                .tcx
                .symbol_name(rustc_middle::ty::Instance::mono(self.tcx, *attr_id))
                .name
                .to_string();
            self.symbol_table.lookup(&shadow_memory_table).cloned()
        };

        let assigns = modified_places
            .into_iter()
            .map(|local| {
                Lambda::as_contract_for(
                    &goto_annotated_fn_typ,
                    None,
                    self.codegen_place_stable(&local.into(), loc).unwrap().goto_expr.dereference(),
                )
            })
            .chain(
                shadow_memory_symbol
                    .map(|shadow_memory_symbol| {
                        vec![Lambda::as_contract_for(
                            &goto_annotated_fn_typ,
                            None,
                            shadow_memory_symbol.to_expr(),
                        )]
                    })
                    .unwrap_or(vec![]),
            )
            .collect();

        FunctionContract::new(assigns)
    }

    /// Convert the contract to a CBMC contract, then attach it to `instance`.
    /// `instance` must have previously been declared.
    ///
    /// This merges with any previously attached contracts.
    pub fn attach_modifies_contract(&mut self, instance: Instance, modified_places: Vec<Local>) {
        // This should be safe, since the contract is pretty much evaluated as
        // though it was the first (or last) assertion in the function.
        assert!(self.current_fn.is_none());
        let body = instance.body().unwrap();
        self.set_current_fn(instance, &body);
        let goto_contract =
            self.codegen_modifies_contract(modified_places, self.codegen_span_stable(body.span));
        let name = self.current_fn().name();

        self.symbol_table.attach_contract(name, goto_contract);
        self.reset_current_fn()
    }
}
