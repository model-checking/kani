// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module responsible for generating code for a few Kani intrinsics.
//!
//! These intrinsics have code that depend on information from the compiler, such as type layout
//! information; thus, they are implemented as a transformation pass where their body get generated
//! by the transformation.

use crate::kani_middle::attributes::matches_diagnostic;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::{
    CheckType, InsertPosition, MutableBody, SourceInstruction,
};
use crate::kani_middle::transform::check_uninit::TypeInfo;
use crate::kani_middle::transform::check_values::{build_limits, ty_validity_per_offset};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    AggregateKind, BinOp, Body, ConstOperand, Operand, Place, Rvalue, Statement, StatementKind,
    RETURN_LOCAL,
};
use stable_mir::target::MachineInfo;
use stable_mir::ty::{GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyConst, TyKind};
use std::fmt::Debug;
use strum_macros::AsRefStr;
use tracing::trace;

/// Generate the body for a few Kani intrinsics.
#[derive(Debug)]
pub struct IntrinsicGeneratorPass {
    pub check_type: CheckType,
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
    fn transform(&self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
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
        new_body.clear_body();

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
                    new_body.new_assignment(rvalue, &mut terminator, InsertPosition::Before);
                let reason = format!(
                    "Kani currently doesn't support checking validity of `{target_ty}`. {msg}"
                );
                new_body.add_check(
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

    fn is_initialized_body(&self, tcx: TyCtxt, body: Body) -> Body {
        let mut new_body = MutableBody::from(body);
        new_body.clear_body();

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

        // The first argument type.
        let arg_ty = new_body.locals()[1].ty;
        let TyKind::RigidTy(RigidTy::RawPtr(target_ty, _)) = arg_ty.kind() else { unreachable!() };
        let type_layout = TypeInfo::from_ty(target_ty).map(|type_info| type_info.get_mask());
        match type_layout {
            Ok(type_layout) => {
                // Given the pointer argument, call the shadow memory
                let layout_operand = Operand::Move(Place {
                    local: new_body.new_assignment(
                        Rvalue::Aggregate(
                            AggregateKind::Array(Ty::bool_ty()),
                            type_layout
                                .as_byte_layout()
                                .iter()
                                .map(|byte| {
                                    Operand::Constant(ConstOperand {
                                        span,
                                        user_ty: None,
                                        const_: MirConst::from_bool(*byte),
                                    })
                                })
                                .collect(),
                        ),
                        &mut terminator,
                        InsertPosition::Before,
                    ),
                    projection: vec![],
                });
                let shadow_memory_get = match target_ty.kind() {
                    TyKind::RigidTy(RigidTy::Slice(_)) | TyKind::RigidTy(RigidTy::Str) => {
                        Instance::resolve(
                            find_fn_def(tcx, "KaniShadowMemoryGetSlice").unwrap(),
                            &GenericArgs(vec![
                                GenericArgKind::Const(
                                    TyConst::try_from_target_usize(
                                        type_layout.as_byte_layout().len() as u64,
                                    )
                                    .unwrap(),
                                ),
                                GenericArgKind::Type(target_ty),
                            ]),
                        )
                        .unwrap()
                    }
                    _ => Instance::resolve(
                        find_fn_def(tcx, "KaniShadowMemoryGet").unwrap(),
                        &GenericArgs(vec![
                            GenericArgKind::Const(
                                TyConst::try_from_target_usize(
                                    type_layout.as_byte_layout().len() as u64
                                )
                                .unwrap(),
                            ),
                            GenericArgKind::Type(target_ty),
                        ]),
                    )
                    .unwrap(),
                };

                // Retrieve current shadow memory info.
                new_body.add_call(
                    &shadow_memory_get,
                    &mut terminator,
                    InsertPosition::Before,
                    vec![
                        Operand::Copy(Place::from(1)),
                        layout_operand,
                        Operand::Copy(Place::from(2)),
                    ],
                    Place::from(ret_var),
                );
            }
            Err(msg) => {
                // We failed to retrieve the type layout.
                let rvalue = Rvalue::Use(Operand::Constant(ConstOperand {
                    const_: MirConst::from_bool(false),
                    span,
                    user_ty: None,
                }));
                let result =
                    new_body.new_assignment(rvalue, &mut terminator, InsertPosition::Before);
                let reason = format!(
                    "Kani currently doesn't support checking memory initialization of `{target_ty}`. {msg}"
                );
                new_body.add_check(
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
