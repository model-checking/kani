use std::io::stdout;
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::transform::BodyTransformation;
use cbmc::goto_program::{DatatypeComponent, FunctionContract, Type};
use cbmc::goto_program::{Lambda, Location};
use cbmc::irep::Symbol;
use kani_metadata::AssignsContract;
use rustc_hir::def_id::DefId as InternalDefId;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::{Body, Local, VarDebugInfoContents};
use stable_mir::ty::{ClosureKind, FnDef, RigidTy, TyKind};
use stable_mir::CrateDef;
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    /// Given the `proof_for_contract` target `function_under_contract` and the reachable `items`,
    /// find or create the `AssignsContract` that needs to be enforced and attach it to the symbol
    /// for which it needs to be enforced.
    ///
    /// 1. Gets the `#[kanitool::inner_check = "..."]` target, then resolves exactly one instance
    ///    of it. Panics if there are more or less than one instance.
    /// 2. The additional arguments for the inner checks are locations that may be modified.
    ///    Add them to the list of CBMC's assigns.
    /// 3. Returns the mangled name of the symbol it attached the contract to.
    /// 4. Returns the full path to the static marked with `#[kanitool::recursion_tracker]` which
    ///    is passed to the `--nondet-static-exclude` argument.
    ///    This flag expects the file path that `checked_with` is located in, the name of the
    ///    `checked_with` function and the name of the constant (`REENTRY`).
    pub fn handle_check_contract(
        &mut self,
        function_under_contract: InternalDefId,
        items: &[MonoItem],
    ) -> AssignsContract {
        let tcx = self.tcx;
        let (check, modify) = items
            .iter()
            .find_map(|item| {
                // Find the instance under contract
                let MonoItem::Fn(instance) = *item else { return None };
                if rustc_internal::internal(tcx, instance.def.def_id()) == function_under_contract {
                    tracing::error!(name=?instance.name(), "------ here");
                    self.find_check_and_modifies(instance)
                } else {
                    None
                }
            })
            .unwrap();
        self.attach_modifies_contract(modify);
        let recursion_tracker = self.find_recursion_tracker(items);
        AssignsContract { recursion_tracker, contracted_function_name: modify.mangled_name() }
    }

    /// The name and location for the recursion tracker should match the exact information added
    /// to the symbol table, otherwise our contract instrumentation will silently failed.
    /// This happens because Kani relies on `--nondet-static-exclude` from CBMC to properly
    /// handle this tracker. CBMC silently fails if there is no match in the symbol table
    /// that correspond to the argument of this flag.
    /// More details at https://github.com/model-checking/kani/pull/3045.
    ///
    /// We must use the pretty name of the tracker instead of the mangled name.
    /// This restriction comes from `--nondet-static-exclude` in CBMC.
    /// Mode details at https://github.com/diffblue/cbmc/issues/8225.
    fn find_recursion_tracker(&mut self, items: &[MonoItem]) -> Option<String> {
        // Return item tagged with `#[kanitool::recursion_tracker]`
        let mut recursion_trackers = items.iter().filter_map(|item| {
            let MonoItem::Static(static_item) = item else { return None };
            if !static_item
                .attrs_by_path(&["kanitool".into(), "recursion_tracker".into()])
                .is_empty()
            {
                let span = static_item.span();
                let loc = self.codegen_span_stable(span);
                Some(format!(
                    "{}:{}",
                    loc.filename().expect("recursion location wrapper should have a file name"),
                    static_item.name(),
                ))
            } else {
                None
            }
        });

        let recursion_tracker = recursion_trackers.next();
        assert!(
            recursion_trackers.next().is_none(),
            "Expected up to one recursion tracker (`REENTRY`) in scope"
        );
        recursion_tracker
    }

    /// Find the modifies and the check closure recursively since we may have a recursion wrapper.
    fn find_check_and_modifies(&mut self, instance: Instance) -> Option<(Instance, Instance)> {
        let contract_attrs =
            KaniAttributes::for_instance(self.tcx, instance).contract_attributes()?;
        let mut find_closure = |inside: Instance, name: &str| {
            let body = self.transformer.body(self.tcx, inside);
            body.var_debug_info.iter().find_map(|var_info| {
                if var_info.name.as_str() == name {
                    let ty = match &var_info.value {
                        VarDebugInfoContents::Place(place) => place.ty(body.locals()).unwrap(),
                        VarDebugInfoContents::Const(const_op) => const_op.ty(),
                    };
                    if let TyKind::RigidTy(RigidTy::Closure(def, args)) = ty.kind() {
                        return Some(Instance::resolve(FnDef(def.def_id()), &args).unwrap());
                    }
                }
                None
            })
        };
        let outside_check = if contract_attrs.has_recursion {
            find_closure(instance, contract_attrs.recursion_check.as_str())?
        } else {
            instance
        };
        let check = find_closure(outside_check, contract_attrs.checked_with.as_str())?;
        Some((check, find_closure(check, contract_attrs.inner_check.as_str())?))
    }

    /// Convert the Kani level contract into a CBMC level contract by creating a
    /// CBMC lambda.
    fn codegen_modifies_contract(
        &mut self,
        goto_annotated_fn_name: &str,
        modifies: Instance,
        loc: Location,
    ) -> FunctionContract {
        let goto_annotated_fn_typ = self
            .symbol_table
            .lookup(goto_annotated_fn_name)
            .unwrap_or_else(|| panic!("Function '{goto_annotated_fn_name}' is not declared"))
            .typ
            .clone();

        let shadow_memory_assign = self
            .tcx
            .all_diagnostic_items(())
            .name_to_id
            .get(&rustc_span::symbol::Symbol::intern("KaniMemInitShadowMem"))
            .map(|attr_id| {
                self.tcx
                    .symbol_name(rustc_middle::ty::Instance::mono(self.tcx, *attr_id))
                    .name
                    .to_string()
            })
            .and_then(|shadow_memory_table| self.symbol_table.lookup(&shadow_memory_table).cloned())
            .map(|shadow_memory_symbol| {
                vec![Lambda::as_contract_for(
                    &goto_annotated_fn_typ,
                    None,
                    shadow_memory_symbol.to_expr(),
                )]
            })
            .unwrap_or_default();

        // The last argument is a tuple with addresses that can be modified.
        let capture_local = Local::from(modifies.fn_abi().unwrap().args.len());
        let capture_ty = self.local_ty_stable(capture_local);
        let captured_args =
            self.codegen_place_stable(&capture_local.into(), loc).unwrap().goto_expr;
        let TyKind::RigidTy(RigidTy::Tuple(capture_tys)) = capture_ty.kind() else {
            unreachable!("found {:?}", capture_ty.kind())
        };
        let assigns = capture_tys
            .into_iter()
            .enumerate()
            .filter_map(|(idx, ty)| {
                let kind = ty.kind();
                kind.is_raw_ptr().then(|| {
                    Lambda::as_contract_for(
                        &goto_annotated_fn_typ,
                        None,
                        captured_args
                            .clone()
                            .member(idx.to_string(), &self.symbol_table)
                            .dereference(),
                    )
                })
            })
            .chain(shadow_memory_assign)
            .collect();

        FunctionContract::new(assigns)
    }

    /// Convert the contract to a CBMC contract, then attach it to `instance`.
    /// `instance` must have previously been declared.
    ///
    /// This merges with any previously attached contracts.
    pub fn attach_modifies_contract(&mut self, instance: Instance) {
        // This should be safe, since the contract is pretty much evaluated as
        // though it was the first (or last) assertion in the function.
        assert!(self.current_fn.is_none());
        let body = self.transformer.body(self.tcx, instance);
        self.set_current_fn(instance, &body);
        let mangled_name = instance.mangled_name();
        let goto_contract = self.codegen_modifies_contract(
            &mangled_name,
            instance,
            self.codegen_span_stable(instance.def.span()),
        );
        self.symbol_table.attach_contract(&mangled_name, goto_contract);
        self.reset_current_fn();
    }
}
