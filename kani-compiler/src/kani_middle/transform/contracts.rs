// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass to enable contracts.
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::transform::body::MutableBody;
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use cbmc::{InternString, InternedString};
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Body, ConstOperand, Operand, TerminatorKind};
use stable_mir::ty::{FnDef, MirConst, RigidTy, TyKind, TypeAndMut};
use stable_mir::{CrateDef, DefId};
use std::collections::HashSet;
use std::fmt::Debug;
use tracing::{debug, trace};

/// Check if we can replace calls to any_modifies.
///
/// This pass will replace the entire body, and it should only be applied to stubs
/// that have a body.
#[derive(Debug)]
pub struct AnyModifiesPass {
    kani_any: Option<FnDef>,
    kani_any_modifies: Option<FnDef>,
    kani_write_any: Option<FnDef>,
    kani_write_any_slim: Option<FnDef>,
    kani_write_any_slice: Option<FnDef>,
    kani_write_any_str: Option<FnDef>,
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
        // TODO: Check if this is the harness has proof_for_contract
        query_db.args().unstable_features.contains(&"function-contracts".to_string())
            && self.kani_any.is_some()
    }

    /// Transform the function body by replacing it with the stub body.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "AnyModifiesPass::transform");

        if instance.def.def_id() == self.kani_any.unwrap().def_id() {
            // Ensure kani::any is valid.
            self.any_body(tcx, body)
        } else if self.should_apply(tcx, instance) {
            // Replace any modifies occurrences.
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
        let kani_write_any = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAny"))
            .map(item_fn_def);
        let kani_write_any_slim = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAnySlim"))
            .map(item_fn_def);
        let kani_write_any_slice = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAnySlice"))
            .map(item_fn_def);
        let kani_write_any_str = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAnyStr"))
            .map(item_fn_def);
        let (target_fn, stubbed) = if let Some(harness) = unit.harnesses.first() {
            let attributes = KaniAttributes::for_instance(tcx, *harness);
            let target_fn =
                attributes.proof_for_contract().map(|symbol| symbol.unwrap().as_str().intern());
            (target_fn, unit.stubs.keys().map(|from| from.def_id()).collect::<HashSet<_>>())
        } else {
            (None, HashSet::new())
        };
        AnyModifiesPass {
            kani_any,
            kani_any_modifies,
            kani_write_any,
            kani_write_any_slim,
            kani_write_any_slice,
            kani_write_any_str,
            target_fn,
            stubbed,
        }
    }

    /// If we apply `transform_any_modifies` in all contract-generated items,
    /// we will end up instantiating `kani::any_modifies` for the replace function
    /// every time, even if we are only checking the contract, because the function
    /// is always included during contract instrumentation. Thus, we must only apply
    /// the transformation if we are using a verified stub or in the presence of recursion.
    fn should_apply(&self, tcx: TyCtxt, instance: Instance) -> bool {
        let item_attributes =
            KaniAttributes::for_item(tcx, rustc_internal::internal(tcx, instance.def.def_id()));
        self.stubbed.contains(&instance.def.def_id()) || item_attributes.has_recursion()
    }

    /// Replace calls to `any_modifies` by calls to `any`.
    fn replace_any_modifies(&self, mut body: Body) -> (bool, Body) {
        let mut changed = false;
        let locals = body.locals().to_vec();
        for bb in body.blocks.iter_mut() {
            let TerminatorKind::Call { func, args, .. } = &mut bb.terminator.kind else { continue };
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

            if let TyKind::RigidTy(RigidTy::FnDef(def, instance_args)) =
                func.ty(&locals).unwrap().kind()
                && Some(def) == self.kani_write_any
                && args.len() == 1
                && let Some(fn_sig) = func.ty(&locals).unwrap().kind().fn_sig()
                && let Some(TypeAndMut { ty: internal_type, mutability: _ }) =
                    fn_sig.skip_binder().inputs()[0].kind().builtin_deref(true)
            {
                if let TyKind::RigidTy(RigidTy::Slice(_)) = internal_type.kind() {
                    let instance =
                        Instance::resolve(self.kani_write_any_slice.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                } else if let TyKind::RigidTy(RigidTy::Str) = internal_type.kind() {
                    let instance =
                        Instance::resolve(self.kani_write_any_str.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                } else {
                    let instance =
                        Instance::resolve(self.kani_write_any_slim.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                }
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
            new_body.clear_body();
            (false, new_body.into())
        }
    }
}
