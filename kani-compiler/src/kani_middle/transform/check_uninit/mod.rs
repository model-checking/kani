// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a transformation pass that instruments the code to detect possible UB due to
//! the accesses to uninitialized memory.

use crate::args::ExtraChecks;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::{
    CheckType, InsertPosition, MutableBody, SourceInstruction,
};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use itertools::Itertools;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{AggregateKind, Body, Constant, Mutability, Operand, Place, Rvalue};
use stable_mir::ty::{Const, GenericArgKind, GenericArgs, RigidTy, Ty, TyKind, UintTy};
use std::fmt::Debug;
use tracing::{debug, trace};

mod ty_layout;
mod uninit_visitor;

use ty_layout::TypeLayout;
use uninit_visitor::{CheckUninitVisitor, InitRelevantInstruction, SourceOp};

const UNINIT_ALLOWLIST: &[&str] = &[
    "kani::shadow::__kani_global_sm_get_inner",
    "kani::shadow::__kani_global_sm_set_inner",
    "kani::shadow::__kani_global_sm_get",
    "kani::shadow::__kani_global_sm_set",
    "kani::shadow::__kani_global_sm_get_slice",
    "kani::shadow::__kani_global_sm_set_slice",
    "std::alloc::alloc",
];

/// Instrument the code with checks for uninitialized memory.
#[derive(Debug)]
pub struct UninitPass {
    pub check_type: CheckType,
}

impl TransformPass for UninitPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Instrumentation
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        let args = query_db.args();
        args.ub_check.contains(&ExtraChecks::Uninit)
    }

    fn transform(&self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");

        let name_without_generics = instance
            .name()
            .split("::")
            .filter(|chunk| !chunk.starts_with("<") && !chunk.ends_with(">"))
            .join("::");
        if UNINIT_ALLOWLIST.iter().any(|allowlist_item| name_without_generics == *allowlist_item) {
            return (false, body);
        }

        let mut new_body = MutableBody::from(body);
        let orig_len = new_body.blocks().len();
        // Do not cache body.blocks().len() since it will change as we add new checks.
        let mut bb_idx = 0;
        while bb_idx < new_body.blocks().len() {
            if let Some(candidate) =
                CheckUninitVisitor::find_next(&new_body, bb_idx, new_body.skip_first(bb_idx))
            {
                self.build_check(tcx, &mut new_body, candidate);
                bb_idx += 1
            } else {
                bb_idx += 1;
            };
        }
        (orig_len != new_body.blocks().len(), new_body.into())
    }
}

impl UninitPass {
    fn build_check(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        instruction: InitRelevantInstruction,
    ) {
        debug!(?instruction, "build_check");
        let (operations_before, operations_after): (Vec<_>, Vec<_>) =
            instruction.operations.into_iter().partition(|operation| match operation {
                SourceOp::Get { .. }
                | SourceOp::BlessDrop { .. }
                | SourceOp::Unsupported { .. } => true,
                SourceOp::Set { .. } => false,
            });
        let operations: Vec<_> =
            vec![operations_before, operations_after].into_iter().flatten().collect();

        let mut source = instruction.source;
        for operation in operations {
            match &operation {
                SourceOp::Unsupported { unsupported_instruction, place } => {
                    let place_ty = place.ty(body.locals()).unwrap();
                    let reason = format!(
                        "Kani currently doesn't support checking memory initialization using instruction `{unsupported_instruction}` for type `{place_ty}`",
                    );
                    self.unsupported_check(tcx, body, &mut source, &reason);
                    continue;
                }
                _ => {}
            };

            let insert_position = match &operation {
                SourceOp::Get { .. } => InsertPosition::Before,
                SourceOp::Set { .. } => InsertPosition::After,
                SourceOp::BlessDrop { .. } => InsertPosition::Before,
                SourceOp::Unsupported { .. } => unreachable!(),
            };

            let place = match &operation {
                SourceOp::Get { place, .. } | SourceOp::Set { place, .. } => place,
                SourceOp::BlessDrop { place, .. } => &Place {
                    local: body.new_assignment(
                        Rvalue::AddressOf(Mutability::Not, place.clone()),
                        &mut source,
                        insert_position,
                    ),
                    projection: vec![],
                },
                SourceOp::Unsupported { .. } => unreachable!(),
            };

            let place_ty = place.ty(body.locals()).unwrap();
            let pointee_ty = match place_ty.kind() {
                TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) => pointee_ty,
                _ => {
                    unreachable!(
                        "Should only build checks for raw pointers, `{place_ty}` encountered"
                    )
                }
            };

            let type_layout = match TypeLayout::get_mask(pointee_ty) {
                Ok(type_layout) => type_layout,
                Err(err) => {
                    let place_ty = place.ty(body.locals()).unwrap();
                    let reason = format!(
                        "Kani currently doesn't support checking memory initialization using instruction for type `{place_ty}` due to the following: `{err}`",
                    );
                    self.unsupported_check(tcx, body, &mut source, &reason);
                    continue;
                }
            };

            let count = match &operation {
                SourceOp::Get { count, .. }
                | SourceOp::Set { count, .. }
                | SourceOp::BlessDrop { count, .. } => count.clone(),
                SourceOp::Unsupported { .. } => unreachable!(),
            };

            let span = source.span(body.blocks());
            let layout_place = Place {
                local: body.new_assignment(
                    Rvalue::Aggregate(
                        AggregateKind::Array(Ty::bool_ty()),
                        type_layout
                            .as_byte_layout()
                            .iter()
                            .map(|byte| {
                                Operand::Constant(Constant {
                                    span,
                                    user_ty: None,
                                    literal: Const::from_bool(*byte),
                                })
                            })
                            .collect(),
                    ),
                    &mut source,
                    insert_position,
                ),
                projection: vec![],
            };

            let ptr_local = match pointee_ty.kind() {
                TyKind::RigidTy(RigidTy::Slice(_)) | TyKind::RigidTy(RigidTy::Str) => body
                    .new_cast_transmute(
                        Operand::Copy(place.clone()),
                        Ty::from_rigid_kind(RigidTy::Slice(Ty::new_tuple(&[]))),
                        Mutability::Not,
                        &mut source,
                        insert_position,
                    ),
                _ => body.new_cast_ptr(
                    Operand::Copy(place.clone()),
                    Ty::new_tuple(&[]),
                    Mutability::Not,
                    &mut source,
                    insert_position,
                ),
            };

            match operation {
                SourceOp::Get { .. } => {
                    let shadow_memory_get = match pointee_ty.kind() {
                        TyKind::RigidTy(RigidTy::Slice(_)) | TyKind::RigidTy(RigidTy::Str) => {
                            Instance::resolve(
                                find_fn_def(tcx, "KaniShadowMemoryGetSlice").unwrap(),
                                &GenericArgs(vec![GenericArgKind::Const(
                                    Const::try_from_uint(
                                        type_layout.as_byte_layout().len() as u128,
                                        UintTy::Usize,
                                    )
                                    .unwrap(),
                                )]),
                            )
                            .unwrap()
                        }
                        _ => Instance::resolve(
                            find_fn_def(tcx, "KaniShadowMemoryGet").unwrap(),
                            &GenericArgs(vec![
                                GenericArgKind::Const(
                                    Const::try_from_uint(
                                        type_layout.as_byte_layout().len() as u128,
                                        UintTy::Usize,
                                    )
                                    .unwrap(),
                                ),
                                GenericArgKind::Type(pointee_ty),
                            ]),
                        )
                        .unwrap(),
                    };

                    let ret_place = Place {
                        local: body.new_local(
                            Ty::bool_ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ),
                        projection: vec![],
                    };
                    body.add_call(
                        &shadow_memory_get,
                        &mut source,
                        insert_position,
                        vec![
                            Operand::Copy(Place { local: ptr_local, projection: vec![] }),
                            Operand::Move(layout_place),
                            count,
                        ],
                        ret_place.clone(),
                    );
                    body.add_check(
                        tcx,
                        &self.check_type,
                        &mut source,
                        insert_position,
                        ret_place.local,
                        &format!("Undefined Behavior: Reading from an uninitialized pointer of type `{place_ty}`"),
                    )
                }
                SourceOp::Set { value, .. } | SourceOp::BlessDrop { value, .. } => {
                    let shadow_memory_set = match pointee_ty.kind() {
                        TyKind::RigidTy(RigidTy::Slice(_)) | TyKind::RigidTy(RigidTy::Str) => {
                            Instance::resolve(
                                find_fn_def(tcx, "KaniShadowMemorySetSlice").unwrap(),
                                &GenericArgs(vec![GenericArgKind::Const(
                                    Const::try_from_uint(
                                        type_layout.as_byte_layout().len() as u128,
                                        UintTy::Usize,
                                    )
                                    .unwrap(),
                                )]),
                            )
                            .unwrap()
                        }
                        _ => Instance::resolve(
                            find_fn_def(tcx, "KaniShadowMemorySet").unwrap(),
                            &GenericArgs(vec![
                                GenericArgKind::Const(
                                    Const::try_from_uint(
                                        type_layout.as_byte_layout().len() as u128,
                                        UintTy::Usize,
                                    )
                                    .unwrap(),
                                ),
                                GenericArgKind::Type(pointee_ty),
                            ]),
                        )
                        .unwrap(),
                    };
                    let ret_place = Place {
                        local: body.new_local(
                            Ty::new_tuple(&[]),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ),
                        projection: vec![],
                    };
                    body.add_call(
                        &shadow_memory_set,
                        &mut source,
                        insert_position,
                        vec![
                            Operand::Copy(Place { local: ptr_local, projection: vec![] }),
                            Operand::Move(layout_place),
                            count,
                            Operand::Constant(Constant {
                                span,
                                user_ty: None,
                                literal: Const::from_bool(value),
                            }),
                        ],
                        ret_place,
                    );
                }
                SourceOp::Unsupported { .. } => {
                    unreachable!()
                }
            }
        }
    }

    fn unsupported_check(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        reason: &str,
    ) {
        let span = source.span(body.blocks());
        let rvalue = Rvalue::Use(Operand::Constant(Constant {
            literal: Const::from_bool(false),
            span,
            user_ty: None,
        }));
        let result = body.new_assignment(rvalue, source, InsertPosition::Before);
        body.add_check(tcx, &self.check_type, source, InsertPosition::Before, result, reason);
    }
}
