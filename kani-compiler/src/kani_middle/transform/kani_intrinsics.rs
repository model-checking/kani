// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module responsible for generating code for a few Kani intrinsics.
//!
//! These intrinsics have code that depend on information from the compiler, such as type layout
//! information; thus, they are implemented as a transformation pass where their body get generated
//! by the transformation.

use crate::args::{Arguments, ExtraChecks};
use crate::kani_middle::attributes::matches_diagnostic;
use crate::kani_middle::transform::body::{
    CheckType, InsertPosition, MutableBody, SourceInstruction,
};
use crate::kani_middle::transform::check_uninit::PointeeInfo;
use crate::kani_middle::transform::check_values::{build_limits, ty_validity_per_offset};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    BinOp, Body, ConstOperand, Operand, Place, Rvalue, Statement, StatementKind, TerminatorKind,
    RETURN_LOCAL,
};
use stable_mir::target::MachineInfo;
use stable_mir::ty::{FnDef, MirConst, RigidTy, Ty, TyKind, UintTy};
use std::collections::HashMap;
use std::fmt::Debug;
use strum_macros::AsRefStr;
use tracing::trace;

use super::check_uninit::{
    get_mem_init_fn_def, mk_layout_operand, resolve_mem_init_fn, PointeeLayout,
};

/// Generate the body for a few Kani intrinsics.
#[derive(Debug)]
pub struct IntrinsicGeneratorPass {
    pub check_type: CheckType,
    /// Used to cache FnDef lookups of injected memory initialization functions.
    pub mem_init_fn_cache: HashMap<&'static str, FnDef>,
    /// Used to enable intrinsics depending on the flags passed.
    pub arguments: Arguments,
}

impl TransformPass for IntrinsicGeneratorPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Instrumentation
    }

    fn is_enabled(&self, _query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Transform the function body by inserting checks one-by-one.
    /// For every unsafe dereference or a transmute operation, we check all values are valid.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");
        if matches_diagnostic(tcx, instance.def, Intrinsics::KaniValidValue.as_ref()) {
            (true, self.valid_value_body(tcx, body))
        } else if matches_diagnostic(tcx, instance.def, Intrinsics::KaniIsInitialized.as_ref()) {
            (true, self.is_initialized_body(tcx, body))
        } else {
            (false, body)
        }
    }
}

impl IntrinsicGeneratorPass {
    /// Generate the body for valid value. Which should be something like:
    ///
    /// ```
    /// pub fn has_valid_value<T>(ptr: *const T) -> bool {
    ///     let mut ret = true;
    ///     let bytes = ptr as *const u8;
    ///     for req in requirements {
    ///         ret &= in_range(bytes, req);
    ///     }
    ///     ret
    /// }
    /// ```
    fn valid_value_body(&self, tcx: TyCtxt, body: Body) -> Body {
        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Return);

        // Initialize return variable with True.
        let ret_var = RETURN_LOCAL;
        let mut terminator = SourceInstruction::Terminator { bb: 0 };
        let span = new_body.locals()[ret_var].span;
        let assign = StatementKind::Assign(
            Place::from(ret_var),
            Rvalue::Use(Operand::Constant(ConstOperand {
                span,
                user_ty: None,
                const_: MirConst::from_bool(true),
            })),
        );
        let stmt = Statement { kind: assign, span };
        new_body.insert_stmt(stmt, &mut terminator, InsertPosition::Before);
        let machine_info = MachineInfo::target();

        // The first and only argument type.
        let arg_ty = new_body.locals()[1].ty;
        let TyKind::RigidTy(RigidTy::RawPtr(target_ty, _)) = arg_ty.kind() else { unreachable!() };
        let validity = ty_validity_per_offset(&machine_info, target_ty, 0);
        match validity {
            Ok(ranges) if ranges.is_empty() => {
                // Nothing to check
            }
            Ok(ranges) => {
                // Given the pointer argument, check for possible invalid ranges.
                let rvalue = Rvalue::Use(Operand::Move(Place::from(1)));
                for range in ranges {
                    let result =
                        build_limits(&mut new_body, &range, rvalue.clone(), &mut terminator);
                    let rvalue = Rvalue::BinaryOp(
                        BinOp::BitAnd,
                        Operand::Move(Place::from(ret_var)),
                        Operand::Move(Place::from(result)),
                    );
                    let assign = StatementKind::Assign(Place::from(ret_var), rvalue);
                    let stmt = Statement { kind: assign, span };
                    new_body.insert_stmt(stmt, &mut terminator, InsertPosition::Before);
                }
            }
            Err(msg) => {
                // We failed to retrieve all the valid ranges.
                let rvalue = Rvalue::Use(Operand::Constant(ConstOperand {
                    const_: MirConst::from_bool(false),
                    span,
                    user_ty: None,
                }));
                let result =
                    new_body.insert_assignment(rvalue, &mut terminator, InsertPosition::Before);
                let reason = format!(
                    "Kani currently doesn't support checking validity of `{target_ty}`. {msg}"
                );
                new_body.insert_check(
                    tcx,
                    &self.check_type,
                    &mut terminator,
                    InsertPosition::Before,
                    result,
                    &reason,
                );
            }
        }
        new_body.into()
    }

    /// Generate the body for `is_initialized`, which looks like the following
    ///
    /// ```
    /// pub fn is_initialized<T>(ptr: *const T, len: usize) -> bool {
    ///     let layout = ... // Byte mask representing the layout of T.
    ///     __kani_mem_init_sm_get(ptr, layout, len)
    /// }
    /// ```
    fn is_initialized_body(&mut self, tcx: TyCtxt, body: Body) -> Body {
        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Return);

        // Initialize return variable with True.
        let ret_var = RETURN_LOCAL;
        let mut terminator = SourceInstruction::Terminator { bb: 0 };
        let span = new_body.locals()[ret_var].span;
        let assign = StatementKind::Assign(
            Place::from(ret_var),
            Rvalue::Use(Operand::Constant(ConstOperand {
                span,
                user_ty: None,
                const_: MirConst::from_bool(true),
            })),
        );
        let stmt = Statement { kind: assign, span };
        new_body.insert_stmt(stmt, &mut terminator, InsertPosition::Before);

        if !self.arguments.ub_check.contains(&ExtraChecks::Uninit) {
            // Short-circut if uninitialized memory checks are not enabled.
            return new_body.into();
        }

        // The first argument type.
        let arg_ty = new_body.locals()[1].ty;
        let TyKind::RigidTy(RigidTy::RawPtr(target_ty, _)) = arg_ty.kind() else { unreachable!() };
        let pointee_info = PointeeInfo::from_ty(target_ty);
        match pointee_info {
            Ok(pointee_info) => {
                match pointee_info.layout() {
                    PointeeLayout::Sized { layout } => {
                        if layout.is_empty() {
                            // Encountered a ZST, so we can short-circut here.
                            return new_body.into();
                        }
                        let is_ptr_initialized_instance = resolve_mem_init_fn(
                            get_mem_init_fn_def(
                                tcx,
                                "KaniIsPtrInitialized",
                                &mut self.mem_init_fn_cache,
                            ),
                            layout.len(),
                            *pointee_info.ty(),
                        );
                        let layout_operand = mk_layout_operand(
                            &mut new_body,
                            &mut terminator,
                            InsertPosition::Before,
                            &layout,
                        );
                        new_body.insert_call(
                            &is_ptr_initialized_instance,
                            &mut terminator,
                            InsertPosition::Before,
                            vec![Operand::Copy(Place::from(1)), layout_operand],
                            Place::from(ret_var),
                        );
                    }
                    PointeeLayout::Slice { element_layout } => {
                        // Since `str`` is a separate type, need to differentiate between [T] and str.
                        let (slicee_ty, diagnostic) = match pointee_info.ty().kind() {
                            TyKind::RigidTy(RigidTy::Slice(slicee_ty)) => {
                                (slicee_ty, "KaniIsSlicePtrInitialized")
                            }
                            TyKind::RigidTy(RigidTy::Str) => {
                                (Ty::unsigned_ty(UintTy::U8), "KaniIsStrPtrInitialized")
                            }
                            _ => unreachable!(),
                        };
                        let is_ptr_initialized_instance = resolve_mem_init_fn(
                            get_mem_init_fn_def(tcx, diagnostic, &mut self.mem_init_fn_cache),
                            element_layout.len(),
                            slicee_ty,
                        );
                        let layout_operand = mk_layout_operand(
                            &mut new_body,
                            &mut terminator,
                            InsertPosition::Before,
                            &element_layout,
                        );
                        new_body.insert_call(
                            &is_ptr_initialized_instance,
                            &mut terminator,
                            InsertPosition::Before,
                            vec![Operand::Copy(Place::from(1)), layout_operand],
                            Place::from(ret_var),
                        );
                    }
                    PointeeLayout::TraitObject => {
                        let rvalue = Rvalue::Use(Operand::Constant(ConstOperand {
                            const_: MirConst::from_bool(false),
                            span,
                            user_ty: None,
                        }));
                        let result = new_body.insert_assignment(
                            rvalue,
                            &mut terminator,
                            InsertPosition::Before,
                        );
                        let reason: &str = "Kani does not support reasoning about memory initialization of pointers to trait objects.";

                        new_body.insert_check(
                            tcx,
                            &self.check_type,
                            &mut terminator,
                            InsertPosition::Before,
                            result,
                            &reason,
                        );
                    }
                };
            }
            Err(msg) => {
                // We failed to retrieve the type layout.
                let rvalue = Rvalue::Use(Operand::Constant(ConstOperand {
                    const_: MirConst::from_bool(false),
                    span,
                    user_ty: None,
                }));
                let result =
                    new_body.insert_assignment(rvalue, &mut terminator, InsertPosition::Before);
                let reason = format!(
                    "Kani currently doesn't support checking memory initialization of `{target_ty}`. {msg}"
                );
                new_body.insert_check(
                    tcx,
                    &self.check_type,
                    &mut terminator,
                    InsertPosition::Before,
                    result,
                    &reason,
                );
            }
        }
        new_body.into()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, AsRefStr)]
#[strum(serialize_all = "PascalCase")]
enum Intrinsics {
    KaniValidValue,
    KaniIsInitialized,
}
