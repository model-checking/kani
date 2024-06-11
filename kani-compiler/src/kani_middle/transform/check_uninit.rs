// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a transformation pass that instruments the code to detect possible UB due to
//! the accesses to uninitialized memory.

use crate::args::ExtraChecks;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::{CheckType, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::abi::{FieldsShape, Scalar, TagEncoding, ValueAbi, VariantsShape};
use stable_mir::mir::mono::{Instance, InstanceKind};
use stable_mir::mir::visit::{Location, PlaceContext};
use stable_mir::mir::{
    AggregateKind, BasicBlockIdx, Body, Constant, LocalDecl, MirVisitor, Mutability,
    NonDivergingIntrinsic, Operand, Place, ProjectionElem, Rvalue, Statement, StatementKind,
    Terminator, TerminatorKind,
};
use stable_mir::target::{MachineInfo, MachineSize};
use stable_mir::ty::{
    AdtKind, Const, GenericArgKind, GenericArgs, IndexedVal, RigidTy, Ty, TyKind, UintTy,
};
use stable_mir::CrateDef;
use std::fmt::Debug;
use strum_macros::AsRefStr;
use tracing::{debug, trace};

const UNINIT_ALLOWLIST: &[&str] =
    &["kani::shadow::global_sm_get", "kani::shadow::global_sm_set", "std::alloc::alloc"];

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

        if UNINIT_ALLOWLIST.iter().any(|allowlist_item| instance.name().contains(allowlist_item)) {
            return (false, body);
        }

        let mut new_body = MutableBody::from(body);
        let orig_len = new_body.blocks().len();
        // Do not cache body.blocks().len() since it will change as we add new checks.
        let mut bb_idx = 0;
        while bb_idx < new_body.blocks().len() {
            if let Some(candidate) =
                CheckUninitVisitor::find_next(&new_body, bb_idx, bb_idx >= orig_len)
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
        let mut source = instruction.source;
        for operation in instruction.operations {
            let place = match &operation {
                SourceOp::Get { place, .. }
                | SourceOp::Set { place, .. }
                | SourceOp::GetN { place, .. }
                | SourceOp::SetN { place, .. } => place,
                SourceOp::Unsupported { instruction, place } => {
                    let place_ty = place.ty(body.locals()).unwrap();
                    let reason = format!(
                        "Kani currently doesn't support checking memory initialization using instruction  `{instruction}` for type `{place_ty}`",
                    );
                    self.unsupported_check(tcx, body, &mut source, &reason);
                    continue;
                }
            };

            let place_ty = place.ty(body.locals()).unwrap();
            let pointee_ty = match place_ty.kind() {
                TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) => pointee_ty,
                _ => unreachable!(),
            };

            let layout_mask = layout_mask(pointee_ty).unwrap();

            // if layout_mask.iter().all(|byte| *byte) {
            //     match operation {
            //         SourceOp::Get { .. } => {
            //             let sm_get = Instance::resolve(
            //                 find_fn_def(tcx, "KaniShadowMemoryGet").unwrap(),
            //                 &GenericArgs(vec![GenericArgKind::Type(pointee_ty)]),
            //             )
            //             .unwrap();

            //             let ret_place = Place {
            //                 local: body.new_local(
            //                     Ty::bool_ty(),
            //                     source.span(body.blocks()),
            //                     Mutability::Not,
            //                 ),
            //                 projection: vec![],
            //             };

            //             body.add_call(
            //                 &sm_get,
            //                 &mut source,
            //                 vec![Operand::Copy(place.clone())],
            //                 ret_place.clone(),
            //             );
            //             body.add_check(
            //                 tcx,
            //                 &self.check_type,
            //                 &mut source,
            //                 ret_place.local,
            //                 &format!("Undefined Behavior: Reading from an uninitialized pointer of type `{place_ty}`"),
            //             )
            //         }
            //         SourceOp::Set { value, .. } => {
            //             let sm_set = Instance::resolve(
            //                 find_fn_def(tcx, "KaniShadowMemorySet").unwrap(),
            //                 &GenericArgs(vec![GenericArgKind::Type(pointee_ty)]),
            //             )
            //             .unwrap();
            //             let ret_place = Place {
            //                 local: body.new_local(
            //                     Ty::new_tuple(&[]),
            //                     source.span(body.blocks()),
            //                     Mutability::Not,
            //                 ),
            //                 projection: vec![],
            //             };
            //             let span = source.span(body.blocks());
            //             body.add_call(
            //                 &sm_set,
            //                 &mut source,
            //                 vec![
            //                     Operand::Copy(place.clone()),
            //                     Operand::Constant(Constant {
            //                         span,
            //                         user_ty: None,
            //                         literal: Const::from_bool(value),
            //                     }),
            //                 ],
            //                 ret_place,
            //             );
            //         }
            //         SourceOp::Unsupported { .. } => {
            //             unreachable!()
            //         }
            //     }
            // } else {
            let span = source.span(body.blocks());
            let layout_local = body.new_assignment(
                Rvalue::Aggregate(
                    AggregateKind::Array(Ty::bool_ty()),
                    layout_mask
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
            );
            let ptr_local = body.new_cast_ptr(
                Operand::Copy(place.clone()),
                Ty::new_tuple(&[]),
                Mutability::Not,
                &mut source,
            );

            match operation {
                SourceOp::Get { .. } => {
                    let sm_get = Instance::resolve(
                        find_fn_def(tcx, "KaniShadowMemoryGetWithLayout").unwrap(),
                        &GenericArgs(vec![GenericArgKind::Const(
                            Const::try_from_uint(layout_mask.len() as u128, UintTy::Usize).unwrap(),
                        )]),
                    )
                    .unwrap();

                    let ret_place = Place {
                        local: body.new_local(
                            Ty::bool_ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ),
                        projection: vec![],
                    };

                    body.add_call(
                        &sm_get,
                        &mut source,
                        vec![
                            Operand::Copy(Place { local: ptr_local, projection: vec![] }),
                            Operand::Move(Place { local: layout_local, projection: vec![] }),
                        ],
                        ret_place.clone(),
                    );
                    body.add_check(
                        tcx,
                        &self.check_type,
                        &mut source,
                        ret_place.local,
                        &format!("Undefined Behavior: Reading from an uninitialized pointer of type `{place_ty}`"),
                    )
                }
                SourceOp::GetN { count, .. } => {
                    let sm_get = Instance::resolve(
                        find_fn_def(tcx, "KaniShadowMemoryGetWithLayoutDynamic").unwrap(),
                        &GenericArgs(vec![GenericArgKind::Const(
                            Const::try_from_uint(layout_mask.len() as u128, UintTy::Usize).unwrap(),
                        )]),
                    )
                    .unwrap();

                    let ret_place = Place {
                        local: body.new_local(
                            Ty::bool_ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ),
                        projection: vec![],
                    };

                    body.add_call(
                        &sm_get,
                        &mut source,
                        vec![
                            Operand::Copy(Place { local: ptr_local, projection: vec![] }),
                            Operand::Move(Place { local: layout_local, projection: vec![] }),
                            count,
                        ],
                        ret_place.clone(),
                    );
                    body.add_check(
                        tcx,
                        &self.check_type,
                        &mut source,
                        ret_place.local,
                        &format!("Undefined Behavior: Reading from an uninitialized pointer of type `{place_ty}`"),
                    )
                }
                SourceOp::Set { value, .. } => {
                    let sm_set = Instance::resolve(
                        find_fn_def(tcx, "KaniShadowMemorySetWithLayout").unwrap(),
                        &GenericArgs(vec![GenericArgKind::Const(
                            Const::try_from_uint(layout_mask.len() as u128, UintTy::Usize).unwrap(),
                        )]),
                    )
                    .unwrap();
                    let ret_place = Place {
                        local: body.new_local(
                            Ty::new_tuple(&[]),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ),
                        projection: vec![],
                    };
                    body.add_call(
                        &sm_set,
                        &mut source,
                        vec![
                            Operand::Copy(Place { local: ptr_local, projection: vec![] }),
                            Operand::Move(Place { local: layout_local, projection: vec![] }),
                            Operand::Constant(Constant {
                                span,
                                user_ty: None,
                                literal: Const::from_bool(value),
                            }),
                        ],
                        ret_place,
                    );
                }
                SourceOp::SetN { count, value, .. } => {
                    let sm_set = Instance::resolve(
                        find_fn_def(tcx, "KaniShadowMemorySetWithLayoutDynamic").unwrap(),
                        &GenericArgs(vec![GenericArgKind::Const(
                            Const::try_from_uint(layout_mask.len() as u128, UintTy::Usize).unwrap(),
                        )]),
                    )
                    .unwrap();
                    let ret_place = Place {
                        local: body.new_local(
                            Ty::new_tuple(&[]),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ),
                        projection: vec![],
                    };
                    body.add_call(
                        &sm_set,
                        &mut source,
                        vec![
                            Operand::Copy(Place { local: ptr_local, projection: vec![] }),
                            Operand::Move(Place { local: layout_local, projection: vec![] }),
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
            // }
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
        let result = body.new_assignment(rvalue, source);
        body.add_check(tcx, &self.check_type, source, result, reason);
    }
}

#[derive(AsRefStr, Clone, Debug)]
enum SourceOp {
    Get { place: Place },
    GetN { place: Place, count: Operand },
    Set { place: Place, value: bool },
    SetN { place: Place, count: Operand, value: bool },
    Unsupported { instruction: String, place: Place },
}

#[derive(Clone, Debug)]
struct InitRelevantInstruction {
    /// The instruction that affects the state of the memory.
    source: SourceInstruction,
    /// All memory-related operations in this instructions.
    operations: Vec<SourceOp>,
}

struct CheckUninitVisitor<'a> {
    locals: &'a [LocalDecl],
    /// Whether we should skip the next instruction, since it might've been instrumented already.
    /// When we instrument an instruction, we partition the basic block, and the instruction that
    /// may trigger UB becomes the first instruction of the basic block, which we need to skip
    /// later.
    skip_next: bool,
    /// The instruction being visited at a given point.
    current: SourceInstruction,
    /// The target instruction that should be verified.
    pub target: Option<InitRelevantInstruction>,
    /// The basic block being visited.
    bb: BasicBlockIdx,
}

fn expect_place(op: &Operand) -> &Place {
    match op {
        Operand::Copy(place) | Operand::Move(place) => place,
        Operand::Constant(_) => unreachable!(),
    }
}

fn try_remove_topmost_deref(place: &Place) -> Option<Place> {
    let mut new_place = place.clone();
    if let Some(ProjectionElem::Deref) = new_place.projection.pop() {
        Some(new_place)
    } else {
        None
    }
}

/// Retrieve instance for the given function operand.
///
/// This will panic if the operand is not a function or if it cannot be resolved.
fn expect_instance(locals: &[LocalDecl], func: &Operand) -> Instance {
    let ty = func.ty(locals).unwrap();
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnDef(def, args)) => Instance::resolve(def, &args).unwrap(),
        _ => unreachable!(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DataBytes {
    /// Offset in bytes.
    offset: usize,
    /// Size of this requirement.
    size: MachineSize,
}

fn scalar_ty_size(machine_info: &MachineInfo, ty: Ty) -> Option<DataBytes> {
    let shape = ty.layout().unwrap().shape();
    match shape.abi {
        ValueAbi::Scalar(Scalar::Initialized { value, .. })
        | ValueAbi::ScalarPair(Scalar::Initialized { value, .. }, _) => {
            Some(DataBytes { offset: 0, size: value.size(machine_info) })
        }
        ValueAbi::Scalar(_)
        | ValueAbi::ScalarPair(_, _)
        | ValueAbi::Uninhabited
        | ValueAbi::Vector { .. }
        | ValueAbi::Aggregate { .. } => None,
    }
}

fn ty_layout(
    machine_info: &MachineInfo,
    ty: Ty,
    current_offset: usize,
) -> Result<Vec<DataBytes>, String> {
    let layout = ty.layout().unwrap().shape();
    let ty_size = || {
        if let Some(mut size) = scalar_ty_size(machine_info, ty) {
            size.offset = current_offset;
            vec![size]
        } else {
            vec![]
        }
    };
    match layout.fields {
        FieldsShape::Primitive => Ok(ty_size()),
        FieldsShape::Array { stride, count } if count > 0 => {
            let TyKind::RigidTy(RigidTy::Array(elem_ty, _)) = ty.kind() else { unreachable!() };
            let elem_validity = ty_layout(machine_info, elem_ty, current_offset)?;
            let mut result = vec![];
            if !elem_validity.is_empty() {
                for idx in 0..count {
                    let idx: usize = idx.try_into().unwrap();
                    let elem_offset = idx * stride.bytes();
                    let mut next_validity = elem_validity
                        .iter()
                        .cloned()
                        .map(|mut req| {
                            req.offset += elem_offset;
                            req
                        })
                        .collect::<Vec<_>>();
                    result.append(&mut next_validity)
                }
            }
            Ok(result)
        }
        FieldsShape::Arbitrary { ref offsets } => {
            match ty.kind().rigid().expect(&format!("unexpected type: {ty:?}")) {
                RigidTy::Adt(def, args) => {
                    match def.kind() {
                        AdtKind::Enum => {
                            // Support basic enumeration forms
                            let ty_variants = def.variants();
                            match layout.variants {
                                VariantsShape::Single { index } => {
                                    // Only one variant is reachable. This behaves like a struct.
                                    let fields = ty_variants[index.to_index()].fields();
                                    let mut fields_validity = vec![];
                                    for idx in layout.fields.fields_by_offset_order() {
                                        let field_offset = offsets[idx].bytes();
                                        let field_ty = fields[idx].ty_with_args(&args);
                                        fields_validity.append(&mut ty_layout(
                                            machine_info,
                                            field_ty,
                                            field_offset + current_offset,
                                        )?);
                                    }
                                    Ok(fields_validity)
                                }
                                VariantsShape::Multiple {
                                    tag_encoding: TagEncoding::Niche { .. },
                                    ..
                                } => {
                                    Err(format!("Unsupported Enum `{}` check", def.trimmed_name()))?
                                }
                                VariantsShape::Multiple { variants, .. } => {
                                    let enum_validity = ty_size();
                                    let mut fields_validity = vec![];
                                    for (index, variant) in variants.iter().enumerate() {
                                        let fields = ty_variants[index].fields();
                                        for field_idx in variant.fields.fields_by_offset_order() {
                                            let field_offset = offsets[field_idx].bytes();
                                            let field_ty = fields[field_idx].ty_with_args(&args);
                                            fields_validity.append(&mut ty_layout(
                                                machine_info,
                                                field_ty,
                                                field_offset + current_offset,
                                            )?);
                                        }
                                    }
                                    if fields_validity.is_empty() {
                                        Ok(enum_validity)
                                    } else {
                                        Err(format!(
                                            "Unsupported Enum `{}` check",
                                            def.trimmed_name()
                                        ))
                                    }
                                }
                            }
                        }
                        AdtKind::Union => unreachable!(),
                        AdtKind::Struct => {
                            // If the struct range has niche add that.
                            let mut struct_validity = ty_size();
                            let fields = def.variants_iter().next().unwrap().fields();
                            for idx in layout.fields.fields_by_offset_order() {
                                let field_offset = offsets[idx].bytes();
                                let field_ty = fields[idx].ty_with_args(&args);
                                struct_validity.append(&mut ty_layout(
                                    machine_info,
                                    field_ty,
                                    field_offset + current_offset,
                                )?);
                            }
                            Ok(struct_validity)
                        }
                    }
                }
                RigidTy::Pat(base_ty, ..) => {
                    // This is similar to a structure with one field and with niche defined.
                    let mut pat_validity = ty_size();
                    pat_validity.append(&mut ty_layout(machine_info, *base_ty, 0)?);
                    Ok(pat_validity)
                }
                RigidTy::Tuple(tys) => {
                    let mut tuple_validity = vec![];
                    for idx in layout.fields.fields_by_offset_order() {
                        let field_offset = offsets[idx].bytes();
                        let field_ty = tys[idx];
                        tuple_validity.append(&mut ty_layout(
                            machine_info,
                            field_ty,
                            field_offset + current_offset,
                        )?);
                    }
                    Ok(tuple_validity)
                }
                RigidTy::Bool
                | RigidTy::Char
                | RigidTy::Int(_)
                | RigidTy::Uint(_)
                | RigidTy::Float(_)
                | RigidTy::Never => {
                    unreachable!("Expected primitive layout for {ty:?}")
                }
                RigidTy::Str | RigidTy::Slice(_) | RigidTy::Array(_, _) => {
                    unreachable!("Expected array layout for {ty:?}")
                }
                RigidTy::RawPtr(_, _) | RigidTy::Ref(_, _, _) => {
                    // Fat pointer has arbitrary shape.
                    Ok(ty_size())
                }
                RigidTy::FnDef(_, _)
                | RigidTy::FnPtr(_)
                | RigidTy::Closure(_, _)
                | RigidTy::Coroutine(_, _, _)
                | RigidTy::CoroutineWitness(_, _)
                | RigidTy::Foreign(_)
                | RigidTy::Dynamic(_, _, _) => Err(format!("Unsupported {ty:?}")),
            }
        }
        FieldsShape::Union(_) | FieldsShape::Array { .. } => {
            /* Anything is valid */
            Ok(vec![])
        }
    }
}

fn layout_mask(ty: Ty) -> Result<Vec<bool>, String> {
    let ty_layout = ty_layout(&MachineInfo::target(), ty, 0)?;
    let ty_size = ty.layout().unwrap().shape().size.bytes();
    let mut layout_mask = vec![false; ty_size];
    for data_bytes in ty_layout.iter() {
        for i in data_bytes.offset..data_bytes.offset + data_bytes.size.bytes() {
            layout_mask[i] = true;
        }
    }
    Ok(layout_mask)
}

impl<'a> CheckUninitVisitor<'a> {
    fn find_next(
        body: &'a MutableBody,
        bb: BasicBlockIdx,
        skip_first: bool,
    ) -> Option<InitRelevantInstruction> {
        let mut visitor = CheckUninitVisitor {
            locals: body.locals(),
            skip_next: skip_first,
            current: SourceInstruction::Statement { idx: 0, bb },
            target: None,
            bb,
        };
        visitor.visit_basic_block(&body.blocks()[bb]);
        visitor.target
    }

    fn push_target(&mut self, op: SourceOp) {
        let target = self.target.get_or_insert_with(|| InitRelevantInstruction {
            source: self.current,
            operations: vec![],
        });
        target.operations.push(op);
    }
}

impl<'a> MirVisitor for CheckUninitVisitor<'a> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        if self.skip_next {
            self.skip_next = false;
        } else if self.target.is_none() {
            // Leave it as an exhaustive match to be notified when a new kind is added.
            match &stmt.kind {
                StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(copy)) => {
                    // Source is a *const T and it must be initialized.
                    self.push_target(SourceOp::GetN {
                        place: expect_place(&copy.src).clone(),
                        count: copy.count.clone(),
                    });
                    // Destimation is a *mut T so it gets initialized.
                    self.push_target(SourceOp::SetN {
                        place: expect_place(&copy.dst).clone(),
                        count: copy.count.clone(),
                        value: true,
                    });
                    self.super_statement(stmt, location)
                }
                StatementKind::Assign(place, rvalue) => {
                    // First check rvalue.
                    self.visit_rvalue(rvalue, location);
                    // Then check the destination place.
                    if let Some(place_without_deref) = try_remove_topmost_deref(place) {
                        if place_without_deref.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                            self.push_target(SourceOp::Set {
                                place: place_without_deref,
                                value: true,
                            });
                        }
                    }
                }
                StatementKind::Deinit(place) => {
                    self.push_target(SourceOp::Set { place: place.clone(), value: false });
                }
                StatementKind::FakeRead(_, _)
                | StatementKind::SetDiscriminant { .. }
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Retag(_, _)
                | StatementKind::PlaceMention(_)
                | StatementKind::AscribeUserType { .. }
                | StatementKind::Coverage(_)
                | StatementKind::ConstEvalCounter
                | StatementKind::Intrinsic(NonDivergingIntrinsic::Assume(_))
                | StatementKind::Nop => self.super_statement(stmt, location),
            }
        }
        let SourceInstruction::Statement { idx, bb } = self.current else { unreachable!() };
        self.current = SourceInstruction::Statement { idx: idx + 1, bb };
    }
    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if !(self.skip_next || self.target.is_some()) {
            self.current = SourceInstruction::Terminator { bb: self.bb };
            // Leave it as an exhaustive match to be notified when a new kind is added.
            match &term.kind {
                TerminatorKind::Call { func, args, .. } => {
                    self.super_terminator(term, location);
                    let instance = expect_instance(self.locals, func);
                    if instance.kind == InstanceKind::Intrinsic {
                        match instance.intrinsic_name().unwrap().as_str() {
                            "write_bytes" => {
                                assert_eq!(
                                    args.len(),
                                    3,
                                    "Unexpected number of arguments for `write_bytes`"
                                );
                                assert!(matches!(
                                    args[0].ty(self.locals).unwrap().kind(),
                                    TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                ));
                                self.push_target(SourceOp::SetN {
                                    place: expect_place(&args[0]).clone(),
                                    count: args[2].clone(),
                                    value: true,
                                })
                            }
                            "compare_bytes" => {
                                assert_eq!(
                                    args.len(),
                                    3,
                                    "Unexpected number of arguments for `compare_bytes`"
                                );
                                assert!(matches!(
                                    args[0].ty(self.locals).unwrap().kind(),
                                    TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                ));
                                assert!(matches!(
                                    args[1].ty(self.locals).unwrap().kind(),
                                    TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                ));
                                self.push_target(SourceOp::GetN {
                                    place: expect_place(&args[0]).clone(),
                                    count: args[2].clone(),
                                });
                                self.push_target(SourceOp::GetN {
                                    place: expect_place(&args[1]).clone(),
                                    count: args[2].clone(),
                                });
                            }
                            "raw_eq" => {
                                assert_eq!(
                                    args.len(),
                                    3,
                                    "Unexpected number of arguments for `raw_eq`"
                                );
                                assert!(matches!(
                                    args[0].ty(self.locals).unwrap().kind(),
                                    TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Not))
                                ));
                                assert!(matches!(
                                    args[1].ty(self.locals).unwrap().kind(),
                                    TyKind::RigidTy(RigidTy::Ref(_, _, Mutability::Not))
                                ));
                                self.push_target(SourceOp::Get {
                                    place: expect_place(&args[0]).clone(),
                                });
                                self.push_target(SourceOp::Get {
                                    place: expect_place(&args[1]).clone(),
                                });
                            }
                            "transmute" | "transmute_copy" => {
                                unreachable!("Should've been lowered")
                            }
                            _ => {}
                        }
                    }
                }
                TerminatorKind::Goto { .. }
                | TerminatorKind::SwitchInt { .. }
                | TerminatorKind::Resume
                | TerminatorKind::Abort
                | TerminatorKind::Return
                | TerminatorKind::Unreachable
                | TerminatorKind::Drop { .. }
                | TerminatorKind::Assert { .. }
                | TerminatorKind::InlineAsm { .. } => self.super_terminator(term, location),
            }
        }
    }

    fn visit_place(&mut self, place: &Place, ptx: PlaceContext, location: Location) {
        for (idx, elem) in place.projection.iter().enumerate() {
            let intermediate_place =
                Place { local: place.local, projection: place.projection[..idx].to_vec() };
            match elem {
                ProjectionElem::Deref => {
                    let ptr_ty = intermediate_place.ty(self.locals).unwrap();
                    if ptr_ty.kind().is_raw_ptr() {
                        self.push_target(SourceOp::Get { place: intermediate_place.clone() });
                    }
                }
                ProjectionElem::Field(idx, target_ty) => {
                    if target_ty.kind().is_union()
                        && (!ptx.is_mutating() || place.projection.len() > idx + 1)
                    {
                        self.push_target(SourceOp::Unsupported {
                            instruction: "union access".to_string(),
                            place: intermediate_place.clone(),
                        });
                    }
                }
                ProjectionElem::Downcast(_) => {}
                ProjectionElem::OpaqueCast(_) => {}
                ProjectionElem::Subtype(_) => {}
                ProjectionElem::Index(_)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Subslice { .. } => {}
            }
        }
        self.super_place(place, ptx, location)
    }
}
