// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::codegen_cprover_gotoc::{codegen::ty_stable::pointee_type_stable, GotocCtx};
use crate::kani_middle::attributes::{ContractAttributes, KaniAttributes, matches_diagnostic as matches_function};
use cbmc::goto_program::FunctionContract;
use cbmc::goto_program::{Expr, Lambda, Location, Type};
use kani_metadata::AssignsContract;
use rustc_hir::def_id::DefId as InternalDefId;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::{Local, TerminatorKind, VarDebugInfoContents};
use stable_mir::ty::{FnDef, RigidTy, TyKind};
use stable_mir::CrateDef;

impl<'tcx> GotocCtx<'tcx> {
    /// Given the `proof_for_contract` target `function_under_contract` and the reachable `items`,
    /// find or create the `AssignsContract` that needs to be enforced and attach it to the symbol
    /// for which it needs to be enforced.
    ///
    /// 1. Gets the `#[kanitool::modifies_wrapper = "..."]` target, then resolves exactly one
    ///    instance of it. Panics if there are more or less than one instance.
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
        let modify = items
            .iter()
            .find_map(|item| {
                // Find the instance under contract
                let MonoItem::Fn(instance) = *item else { return None };
                if rustc_internal::internal(tcx, instance.def.def_id()) == function_under_contract {
                    self.find_modifies(instance)
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

    fn find_closure(&mut self, inside: Instance, name: &str) -> Option<Instance> {
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
    }

    /// Find the modifies recursively since we may have a recursion wrapper.
    /// I.e.: [recursion_wrapper ->]? check -> modifies.
    fn find_modifies(&mut self, instance: Instance) -> Option<Instance> {
        let contract_attrs =
            KaniAttributes::for_instance(self.tcx, instance).contract_attributes()?;
        let check_instance = if contract_attrs.has_recursion {
            let recursion_check = self.find_closure(instance, contract_attrs.recursion_check.as_str())?;
            self.find_closure(recursion_check, contract_attrs.checked_with.as_str())?
        } else {
            self.find_closure(instance, contract_attrs.checked_with.as_str())?
        };
        self.find_closure(check_instance, contract_attrs.modifies_wrapper.as_str())
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
            .get(&rustc_span::symbol::Symbol::intern("KaniMemoryInitializationState"))
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
        let modifies_local = Local::from(modifies.fn_abi().unwrap().args.len());
        let modifies_ty = self.local_ty_stable(modifies_local);
        let modifies_args =
            self.codegen_place_stable(&modifies_local.into(), loc).unwrap().goto_expr;
        let TyKind::RigidTy(RigidTy::Tuple(modifies_tys)) = modifies_ty.kind() else {
            unreachable!("found {:?}", modifies_ty.kind())
        };

        for ty in &modifies_tys {
            assert!(ty.kind().is_any_ptr(), "Expected pointer, but found {}", ty);
        }

        let assigns: Vec<_> = modifies_tys
            .into_iter()
            // do not attempt to dereference (and assign) a ZST
            .filter(|ty| !self.is_zst_stable(pointee_type_stable(*ty).unwrap()))
            .enumerate()
            .map(|(idx, ty)| {
                let ptr = modifies_args.clone().member(idx.to_string(), &self.symbol_table);
                if self.is_fat_pointer_stable(ty) {
                    let unref = match ty.kind() {
                        TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) => pointee_ty,
                        kind => unreachable!("Expected a raw pointer, but found {:?}", kind),
                    };
                    let size = match unref.kind() {
                        TyKind::RigidTy(RigidTy::Slice(elt_type)) => {
                            elt_type.layout().unwrap().shape().size.bytes()
                        }
                        TyKind::RigidTy(RigidTy::Str) => 1,
                        // For adt, see https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp
                        TyKind::RigidTy(RigidTy::Adt(..)) => {
                            todo!("Adt fat pointers not implemented")
                        }
                        kind => unreachable!("Generating a slice fat pointer to {:?}", kind),
                    };
                    Lambda::as_contract_for(
                        &goto_annotated_fn_typ,
                        None,
                        Expr::symbol_expression(
                            "__CPROVER_object_upto",
                            Type::code(
                                vec![
                                    Type::empty()
                                        .to_pointer()
                                        .as_parameter(None, Some("ptr".into())),
                                    Type::size_t().as_parameter(None, Some("size".into())),
                                ],
                                Type::empty(),
                            ),
                        )
                        .call(vec![
                            ptr.clone()
                                .member("data", &self.symbol_table)
                                .cast_to(Type::empty().to_pointer()),
                            ptr.member("len", &self.symbol_table).mul(Expr::size_constant(
                                size.try_into().unwrap(),
                                &self.symbol_table,
                            )),
                        ]),
                    )
                } else {
                    Lambda::as_contract_for(&goto_annotated_fn_typ, None, ptr.dereference())
                }
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

    /// Count the number of contracts applied to `contracted_function`
    pub fn count_contracts(&mut self, contracted_function: &Instance, contract_attrs: ContractAttributes) -> usize {
        // Extract the body of the check_closure, which will contain kani::assume() calls for each precondition
        // and kani::assert() calls for each postcondition.
        let check_closure = self.find_closure(*contracted_function, contract_attrs.checked_with.as_str()).unwrap();
        let body = check_closure.body().unwrap();

        let mut count = 0;

        for bb in &body.blocks {
            if let TerminatorKind::Call { ref func, .. } = bb.terminator.kind {
                let fn_ty = func.ty(body.locals()).unwrap();
                if let TyKind::RigidTy(RigidTy::FnDef(fn_def, args)) = fn_ty.kind() {
                    let instance = Instance::resolve(fn_def, &args).unwrap();
                    // For each precondition or postcondition, increment the count
                    if matches_function(self.tcx, instance.def, "KaniAssume") 
                    || matches_function(self.tcx, instance.def, "KaniAssert") {
                        count += 1;
                    }
                }
            }
        }
        count
    }
}
