// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass to enable contracts.
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::transform::body::{
    new_move_operand, re_erased, CheckType, InsertPosition, MutableBody, SourceInstruction,
};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use cbmc::{InternString, InternedString};
use rustc_hir::def_id::DefId as InternalDefId;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    AggregateKind, Body, BorrowKind, ConstOperand, Local, Mutability, Operand, Place, Rvalue,
    TerminatorKind, VarDebugInfo, VarDebugInfoContents,
};
use stable_mir::ty::{
    ClosureDef, ClosureKind, FnDef, GenericArgs, MirConst, Region, RigidTy, Ty, TyKind,
};
use stable_mir::{CrateDef, DefId};
use std::collections::HashSet;
use std::fmt::Debug;
use std::io::{stdout, Stdout};
use tracing::{debug, trace};

/// Check if we can replace calls to any_modifies.
///
/// This pass will replace the entire body, and it should only be applied to stubs
/// that have a body.
#[derive(Debug)]
pub struct AnyModifiesPass {
    kani_any: Option<FnDef>,
    kani_any_modifies: Option<FnDef>,
    stubbed: HashSet<DefId>,
    target_fn: Option<InternedString>,
}

impl TransformPass for AnyModifiesPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        // TODO: Check if the harness has proof_for_contract
        query_db.args().unstable_features.contains(&"function-contracts".to_string())
            && self.kani_any.is_some()
    }

    /// Transform the function body by replacing it with the stub body.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "AnyModifiesPass::transform");

        if instance.def.def_id() == self.kani_any.unwrap().def_id() {
            // Ensure kani::any is valid.
            self.any_body(tcx, body)
        } else if instance.ty().kind().is_closure() {
            // Replace any modifies occurrences. They should only happen in the contract closures.
            self.replace_any_modifies(body)
        } else {
            (false, body)
        }
    }
}

impl AnyModifiesPass {
    /// Build the pass with non-extern function stubs.
    pub fn new(tcx: TyCtxt, unit: &CodegenUnit) -> AnyModifiesPass {
        let item_fn_def = |item| {
            let TyKind::RigidTy(RigidTy::FnDef(def, _)) =
                rustc_internal::stable(tcx.type_of(item)).value.kind()
            else {
                unreachable!("Expected function, but found `{:?}`", tcx.def_path_str(item))
            };
            def
        };
        let kani_any =
            tcx.get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniAny")).map(item_fn_def);
        let kani_any_modifies = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniAnyModifies"))
            .map(item_fn_def);
        let (target_fn, stubbed) = if let Some(harness) = unit.harnesses.first() {
            let attributes = KaniAttributes::for_instance(tcx, *harness);
            let target_fn =
                attributes.proof_for_contract().map(|symbol| symbol.unwrap().as_str().intern());
            (target_fn, unit.stubs.keys().map(|from| from.def_id()).collect::<HashSet<_>>())
        } else {
            (None, HashSet::new())
        };
        AnyModifiesPass { kani_any, kani_any_modifies, target_fn, stubbed }
    }

    /// Replace calls to `any_modifies` by calls to `any`.
    fn replace_any_modifies(&self, mut body: Body) -> (bool, Body) {
        let mut changed = false;
        let locals = body.locals().to_vec();
        for bb in body.blocks.iter_mut() {
            let TerminatorKind::Call { func, .. } = &mut bb.terminator.kind else { continue };
            if let TyKind::RigidTy(RigidTy::FnDef(def, instance_args)) =
                func.ty(&locals).unwrap().kind()
                && Some(def) == self.kani_any_modifies
            {
                let instance = Instance::resolve(self.kani_any.unwrap(), &instance_args).unwrap();
                let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                let span = bb.terminator.span;
                let new_func = ConstOperand { span, user_ty: None, const_: literal };
                *func = Operand::Constant(new_func);
                changed = true;
            }
        }
        (changed, body)
    }

    /// Check if T::Arbitrary requirement for `kani::any()` is met after replacement.
    ///
    /// If it T does not implement arbitrary, generate error and delete body to interrupt analysis.
    fn any_body(&self, tcx: TyCtxt, mut body: Body) -> (bool, Body) {
        let mut valid = true;
        let locals = body.locals().to_vec();
        for bb in body.blocks.iter_mut() {
            let TerminatorKind::Call { func, .. } = &mut bb.terminator.kind else { continue };
            if let TyKind::RigidTy(RigidTy::FnDef(def, args)) = func.ty(&locals).unwrap().kind() {
                match Instance::resolve(def, &args) {
                    Ok(_) => {}
                    Err(e) => {
                        valid = false;
                        debug!(?e, "AnyModifiesPass::any_body failed");
                        let receiver_ty = args.0[0].expect_ty();
                        let msg = if self.target_fn.is_some() {
                            format!(
                                "`{receiver_ty}` doesn't implement `kani::Arbitrary`.\
                                        Please, check `{}` contract.",
                                self.target_fn.unwrap(),
                            )
                        } else {
                            format!("`{receiver_ty}` doesn't implement `kani::Arbitrary`.")
                        };
                        tcx.dcx()
                            .struct_span_err(rustc_internal::internal(tcx, bb.terminator.span), msg)
                            .with_help(
                                "All objects in the modifies clause must implement the Arbitrary. \
                                 The return type must also implement the Arbitrary trait if you \
                                 are checking recursion or using verified stub.",
                            )
                            .emit();
                    }
                }
            }
        }
        if valid {
            (true, body)
        } else {
            let mut new_body = MutableBody::from(body);
            new_body.clear_body(TerminatorKind::Unreachable);
            (true, new_body.into())
        }
    }
}

/// This pass will transform functions annotated with contracts based on the harness configuration.
///
/// For functions that are being checked, change the body to invoke the "check" closure.
/// For functions that  are being stubbed, change the body to invoke the "replace" closure.
///
/// Functions with contract contains
#[derive(Debug)]
pub struct FunctionWithContractPass {
    /// Function that is being checked, if any.
    check_fn: Option<InternalDefId>,
    /// Functions that should be stubbed by their contract.
    replace_fns: HashSet<InternalDefId>,
    /// Functions annotated with contract attributes will contain contract closures even if they
    /// are not to be used in this harness.
    /// In order to avoid bringing unnecessary logic, we clear their body.
    unused_closures: HashSet<ClosureDef>,
}

impl TransformPass for FunctionWithContractPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, _query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Transform the function body by replacing it with the stub body.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "FunctionWithContractPass::transform");
        tracing::error!(function=?instance.name(), "FunctionWithContractPass::transform");
        let _ =
            tracing::error_span!("FunctionWithContractPass::transform {}", name = instance.name())
                .entered();
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(def, _args) => {
                if let Some(target_closure) = self.select_closure(tcx, *def, &body) {
                    tracing::error!(?target_closure, "FunctionWithContractPass::transform");
                    let _ = body.dump(&mut stdout(), &instance.name());
                    let new_body = self.replace_by_closure(body, target_closure);
                    let _ = new_body.dump(&mut stdout(), &instance.name());
                    (true, new_body)
                } else {
                    // Not a contract annotated function
                    (false, body)
                }
            }
            RigidTy::Closure(def, _args) => {
                if self.unused_closures.contains(def) {
                    tracing::error!("FunctionWithContractPass::transform delete");
                    // Delete body and mark it as unreachable.
                    let mut new_body = MutableBody::from(body);
                    new_body.clear_body(TerminatorKind::Unreachable);
                    (true, new_body.into())
                } else {
                    // Not a contract annotated function
                    (false, body)
                }
            }
            other => unreachable!("Unexpected instance type: `{other:?}`"),
        }
    }
}

impl FunctionWithContractPass {
    /// Build the pass by collecting which functions we are stubbing and which ones we are
    /// verifying.
    pub fn new(tcx: TyCtxt, unit: &CodegenUnit) -> FunctionWithContractPass {
        let harness = unit.harnesses.first().unwrap();
        let attrs = KaniAttributes::for_instance(tcx, *harness);
        let check_fn = attrs.interpret_for_contract_attribute().map(|(_, def_id, span)| def_id);
        let replace_fns: HashSet<_> = attrs
            .interpret_stub_verified_attribute()
            .iter()
            .map(|(_, def_id, span)| *def_id)
            .collect();
        FunctionWithContractPass { check_fn, replace_fns, unused_closures: Default::default() }
    }

    /// Create the following body:
    ///
    /// fn original([self], args*) {
    ///  bb0: {
    //     _3 = {closure@span} { self: _1 }; # If receiver. Otherwise, skip.
    //     _4 = &_3;     # Closure reference is the first argument of the closure.
    //     _5 = (args*); # Closure arguments tupled is the second argument.
    //     _0 = <{closure@span} as Fn<(u32,)>>::call(move _4, move _5) -> [return: bb1];
    //   }
    //
    //   bb1: {
    //     return;
    //   }
    /// }
    fn replace_by_closure(&self, body: Body, closure: ClosureInfo) -> Body {
        tracing::error!(?closure, "replace_by_closure");
        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Return);
        let mut source = SourceInstruction::Terminator { bb: 0 };

        // 1- Create the closure structure if needed, i.e., if it captures the receiver.
        let closure_ty = closure.ty;
        let captures = !closure_ty.layout().unwrap().shape().is_1zst();
        let closure_local = if captures {
            // This closure captures the receiver.
            let capture = Rvalue::Aggregate(
                AggregateKind::Closure(closure.def, closure.args.clone()),
                vec![new_move_operand(Local::from(1usize))],
            );
            assert_eq!(capture.ty(new_body.locals()), Ok(closure_ty), "Expected to capture `self`");

            new_body.new_assignment(capture, &mut source, InsertPosition::Before)
        } else {
            new_body.new_local(closure_ty, source.span(new_body.blocks()), Mutability::Not)
        };

        // 2- Take the structure address.
        let capture_addr = new_body.new_assignment(
            Rvalue::Ref(re_erased(), BorrowKind::Shared, Place::from(closure_local)),
            &mut source,
            InsertPosition::Before,
        );

        // 3- Create tuple with arguments.
        let arg_start = if captures { 2 } else { 1 };
        let arg_locals: Vec<_> =
            (arg_start..=new_body.arg_count()).map(|l| new_move_operand(l)).collect();
        let tupled_args = new_body.new_assignment(
            Rvalue::Aggregate(AggregateKind::Tuple, arg_locals),
            &mut source,
            InsertPosition::Before,
        );

        // 4- Call closure and store result into `_0`
        let closure_args = vec![new_move_operand(capture_addr), new_move_operand(tupled_args)];
        let closure_instance =
            Instance::resolve_closure(closure.def, &closure.args, ClosureKind::FnOnce).unwrap();
        tracing::error!(mangled=?closure_instance.mangled_name(), "replace_by_closure");
        new_body.add_call(
            &closure_instance,
            &mut source,
            InsertPosition::Before,
            closure_args,
            Place::from(Local::from(0usize)),
        );

        new_body.into()
    }

    /// Select which contract closure to keep, if any, and mark the rest as unused.
    fn select_closure(&mut self, tcx: TyCtxt, fn_def: FnDef, body: &Body) -> Option<ClosureInfo> {
        let kani_attributes = KaniAttributes::for_def_id(tcx, fn_def.def_id());
        if let Some(contract) = kani_attributes.contract_attributes() {
            let recursion_closure =
                find_closure(tcx, fn_def, &body, contract.recursion_check.as_str());
            let check_closure = find_closure(tcx, fn_def, &body, contract.checked_with.as_str());
            let replace_closure = find_closure(tcx, fn_def, &body, contract.replaced_with.as_str());
            let fn_def_id = rustc_internal::internal(tcx, fn_def.def_id());
            if self.check_fn == Some(fn_def_id) {
                // Delete replace closure and one check closure depending on the type.
                self.unused_closures.insert(replace_closure.def);
                if contract.has_recursion {
                    self.unused_closures.insert(check_closure.def);
                    Some(recursion_closure)
                } else {
                    self.unused_closures.insert(recursion_closure.def);
                    Some(check_closure)
                }
            } else if self.replace_fns.contains(&fn_def_id) {
                // Delete the check closures.
                self.unused_closures.insert(recursion_closure.def);
                self.unused_closures.insert(check_closure.def);
                Some(replace_closure)
            } else {
                // No contract instrumentation needed. Add all closures to the list of unused.
                self.unused_closures.insert(recursion_closure.def);
                self.unused_closures.insert(check_closure.def);
                self.unused_closures.insert(replace_closure.def);
                None
            }
        } else {
            // Nothing to do
            None
        }
    }
}

#[derive(Clone, Debug)]
struct ClosureInfo {
    ty: Ty,
    def: ClosureDef,
    args: GenericArgs,
}

fn find_closure(tcx: TyCtxt, fn_def: FnDef, body: &Body, name: &str) -> ClosureInfo {
    body.var_debug_info
        .iter()
        .find_map(|var_info| {
            if var_info.name.as_str() == name {
                let ty = match &var_info.value {
                    VarDebugInfoContents::Place(place) => place.ty(body.locals()).unwrap(),
                    VarDebugInfoContents::Const(const_op) => const_op.ty(),
                };
                if let TyKind::RigidTy(RigidTy::Closure(def, args)) = ty.kind() {
                    return Some(ClosureInfo { ty, def, args });
                }
            }
            None
        })
        .unwrap_or_else(|| {
            tcx.sess.dcx().err(format!(
                "Failed to find contract closure `{name}` in function `{}`",
                fn_def.name()
            ));
            tcx.sess.dcx().abort_if_errors();
            unreachable!()
        })
}
