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
use lazy_static::lazy_static;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{AggregateKind, Body, ConstOperand, Mutability, Operand, Place, Rvalue};
use stable_mir::ty::{FnDef, GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyConst, TyKind};
use stable_mir::CrateDef;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Mutex;
use tracing::{debug, trace};

mod ty_layout;
mod uninit_visitor;

pub use ty_layout::{TypeInfo, TypeLayout};
use uninit_visitor::{CheckUninitVisitor, InitRelevantInstruction, SourceOp};

const SKIPPED_DIAGNOSTIC_ITEMS: &[&str] = &["KaniShadowMemoryGetInner", "KaniShadowMemorySetInner"];

/// Retrieve a function definition by diagnostic string, caching the result.
fn get_kani_sm_function(tcx: TyCtxt, diagnostic: &'static str) -> FnDef {
    lazy_static! {
        static ref KANI_SM_FUNCTIONS: Mutex<HashMap<&'static str, FnDef>> =
            Mutex::new(HashMap::new());
    }
    let mut kani_sm_functions = KANI_SM_FUNCTIONS.lock().unwrap();
    let entry = kani_sm_functions
        .entry(diagnostic)
        .or_insert_with(|| find_fn_def(tcx, diagnostic).unwrap());
    *entry
}

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

        // Need to break infinite recursion when shadow memory checks are inserted,
        // so the internal function responsible for shadow memory checks are skipped.
        if tcx
            .get_diagnostic_name(rustc_internal::internal(tcx, instance.def.def_id()))
            .map(|diagnostic_name| {
                SKIPPED_DIAGNOSTIC_ITEMS.contains(&diagnostic_name.to_ident_string().as_str())
            })
            .unwrap_or(false)
        {
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
                self.build_check_for_instruction(tcx, &mut new_body, candidate);
                bb_idx += 1
            } else {
                bb_idx += 1;
            };
        }
        (orig_len != new_body.blocks().len(), new_body.into())
    }
}

impl UninitPass {
    fn build_check_for_instruction(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        instruction: InitRelevantInstruction,
    ) {
        debug!(?instruction, "build_check");
        let mut source = instruction.source;
        for operation in instruction.before_instruction {
            self.build_check_for_operation(tcx, body, &mut source, operation);
        }
        for operation in instruction.after_instruction {
            self.build_check_for_operation(tcx, body, &mut source, operation);
        }
    }

    fn build_check_for_operation(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: SourceOp,
    ) {
        if let SourceOp::Unsupported { reason, position } = &operation {
            self.unsupported_check(tcx, body, source, *position, reason);
            return;
        };

        let pointee_ty_info = {
            let ptr_operand = operation.mk_operand(body, source);
            let ptr_operand_ty = ptr_operand.ty(body.locals()).unwrap();
            let pointee_ty = match ptr_operand_ty.kind() {
                TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) => pointee_ty,
                _ => {
                    unreachable!(
                        "Should only build checks for raw pointers, `{ptr_operand_ty}` encountered."
                    )
                }
            };
            match TypeInfo::from_ty(pointee_ty) {
                Ok(type_info) => type_info,
                Err(_) => {
                    let reason = format!(
                        "Kani currently doesn't support checking memory initialization for pointers to `{pointee_ty}.",
                    );
                    self.unsupported_check(tcx, body, source, operation.position(), &reason);
                    return;
                }
            }
        };

        match operation {
            SourceOp::Get { .. } => {
                self.build_get_and_check(tcx, body, source, operation, pointee_ty_info)
            }
            SourceOp::Set { value, .. } | SourceOp::SetRef { value, .. } => {
                self.build_set(tcx, body, source, operation, pointee_ty_info, value)
            }
            SourceOp::Unsupported { .. } => {
                unreachable!()
            }
        }
    }

    fn build_get_and_check(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: SourceOp,
        pointee_info: TypeInfo,
    ) {
        let pointee_ty_layout = pointee_info.get_mask();
        // Resolve appropriate function depending on the pointer type.
        let shadow_memory_get = match pointee_info.ty().kind() {
            TyKind::RigidTy(RigidTy::Slice(_)) | TyKind::RigidTy(RigidTy::Str) => {
                Instance::resolve(
                    get_kani_sm_function(tcx, "KaniShadowMemoryGetSlice"),
                    &GenericArgs(vec![
                        GenericArgKind::Const(
                            TyConst::try_from_target_usize(
                                pointee_ty_layout.as_byte_layout().len() as u64,
                            )
                            .unwrap(),
                        ),
                        GenericArgKind::Type(*pointee_info.ty()),
                    ]),
                )
                .unwrap()
            }
            TyKind::RigidTy(RigidTy::Dynamic(..)) => return, // Any layout is valid when dereferencing a pointer to `dyn Trait`.
            _ => Instance::resolve(
                get_kani_sm_function(tcx, "KaniShadowMemoryGet"),
                &GenericArgs(vec![
                    GenericArgKind::Const(
                        TyConst::try_from_target_usize(
                            pointee_ty_layout.as_byte_layout().len() as u64
                        )
                        .unwrap(),
                    ),
                    GenericArgKind::Type(*pointee_info.ty()),
                ]),
            )
            .unwrap(),
        };

        let ret_place = Place {
            local: body.new_local(Ty::bool_ty(), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        // Retrieve current shadow memory info.
        let ptr_operand = operation.mk_operand(body, source);
        let layout_operand =
            mk_layout_operand(body, source, operation.position(), pointee_ty_layout);
        body.add_call(
            &shadow_memory_get,
            source,
            operation.position(),
            vec![ptr_operand.clone(), layout_operand, operation.expect_count()],
            ret_place.clone(),
        );
        // Make sure all non-padding bytes are initialized.
        let ptr_operand_ty = ptr_operand.ty(body.locals()).unwrap();
        body.add_check(
            tcx,
            &self.check_type,
            source,
            operation.position(),
            ret_place.local,
            &format!("Undefined Behavior: Reading from an uninitialized pointer of type `{ptr_operand_ty}`"),
        )
    }

    fn build_set(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: SourceOp,
        pointee_info: TypeInfo,
        value: bool,
    ) {
        let pointee_ty_layout = pointee_info.get_mask();
        // Resolve appropriate function depending on the pointer type.
        let shadow_memory_set = match pointee_info.ty().kind() {
            TyKind::RigidTy(RigidTy::Slice(_)) | TyKind::RigidTy(RigidTy::Str) => {
                Instance::resolve(
                    get_kani_sm_function(tcx, "KaniShadowMemorySetSlice"),
                    &GenericArgs(vec![
                        GenericArgKind::Const(
                            TyConst::try_from_target_usize(
                                pointee_ty_layout.as_byte_layout().len() as u64,
                            )
                            .unwrap(),
                        ),
                        GenericArgKind::Type(*pointee_info.ty()),
                    ]),
                )
                .unwrap()
            }
            TyKind::RigidTy(RigidTy::Dynamic(..)) => return, // Any layout is valid when dereferencing a pointer to `dyn Trait`.
            _ => Instance::resolve(
                get_kani_sm_function(tcx, "KaniShadowMemorySet"),
                &GenericArgs(vec![
                    GenericArgKind::Const(
                        TyConst::try_from_target_usize(
                            pointee_ty_layout.as_byte_layout().len() as u64
                        )
                        .unwrap(),
                    ),
                    GenericArgKind::Type(*pointee_info.ty()),
                ]),
            )
            .unwrap(),
        };
        let ret_place = Place {
            local: body.new_local(Ty::new_tuple(&[]), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        // Initialize all non-padding bytes.
        let ptr_operand = operation.mk_operand(body, source);
        let layout_operand =
            mk_layout_operand(body, source, operation.position(), pointee_ty_layout);
        body.add_call(
            &shadow_memory_set,
            source,
            operation.position(),
            vec![
                ptr_operand,
                layout_operand,
                operation.expect_count(),
                Operand::Constant(ConstOperand {
                    span: source.span(body.blocks()),
                    user_ty: None,
                    const_: MirConst::from_bool(value),
                }),
            ],
            ret_place,
        );
    }

    fn unsupported_check(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        position: InsertPosition,
        reason: &str,
    ) {
        let span = source.span(body.blocks());
        let rvalue = Rvalue::Use(Operand::Constant(ConstOperand {
            const_: MirConst::from_bool(false),
            span,
            user_ty: None,
        }));
        let result = body.new_assignment(rvalue, source, position);
        body.add_check(tcx, &self.check_type, source, position, result, reason);
    }
}

fn mk_layout_operand(
    body: &mut MutableBody,
    source: &mut SourceInstruction,
    position: InsertPosition,
    layout: TypeLayout,
) -> Operand {
    Operand::Move(Place {
        local: body.new_assignment(
            Rvalue::Aggregate(
                AggregateKind::Array(Ty::bool_ty()),
                layout
                    .as_byte_layout()
                    .iter()
                    .map(|byte| {
                        Operand::Constant(ConstOperand {
                            span: source.span(body.blocks()),
                            user_ty: None,
                            const_: MirConst::from_bool(*byte),
                        })
                    })
                    .collect(),
            ),
            source,
            position,
        ),
        projection: vec![],
    })
}
