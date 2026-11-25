// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Visitor that collects all instructions relevant to uninitialized memory access.

use crate::{
    intrinsics::Intrinsic,
    kani_middle::transform::{
        body::{InsertPosition, MutableBody, SourceInstruction},
        check_uninit::{
            PointeeInfo, TargetFinder,
            relevant_instruction::{InitRelevantInstruction, MemoryInitOp},
            ty_layout::{
                PointeeLayout, TransmuteRequirement, transmute_requirement,
                tys_layout_compatible_to_size,
            },
        },
    },
    kani_middle::{nonnull_pointee, points_to::resolve_call},
};
use rustc_middle::ty::TyCtxt;
use rustc_public::{CrateDefType, rustc_internal};
use rustc_public::{
    mir::{
        AggregateKind, BorrowKind, CastKind, Local, LocalDecl, MirVisitor, NonDivergingIntrinsic,
        Operand, Place, PointerCoercion, ProjectionElem, RawPtrKind, Rvalue, Statement,
        StatementKind, Terminator, TerminatorKind,
        alloc::GlobalAlloc,
        mono::{Instance, InstanceKind},
        visit::{Location, PlaceContext},
    },
    ty::{AdtKind, ConstantKind, RigidTy, Ty, TyConstKind, TyKind},
};
use std::collections::HashSet;

pub struct CheckUninitVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    instance: Instance,
    locals: Vec<LocalDecl>,
    /// Locals that only ever hold a pointer read out of a `Box` (see `box_pointer_locals`).
    box_pointers: HashSet<Local>,
    /// All target instructions in the body.
    targets: Vec<InitRelevantInstruction>,
    /// Current analysis target, eventually needs to be added to a list of all targets.
    current_target: InitRelevantInstruction,
}

impl TargetFinder for CheckUninitVisitor<'_> {
    fn find_all(mut self, body: &MutableBody) -> Vec<InitRelevantInstruction> {
        self.locals = body.locals().to_vec();
        self.box_pointers = box_pointer_locals(body, &self.locals);
        for (bb_idx, bb) in body.blocks().iter().enumerate() {
            // Set the first target to start iterating from.
            self.current_target = if !bb.statements.is_empty() {
                InitRelevantInstruction {
                    source: SourceInstruction::Statement { idx: 0, bb: bb_idx },
                    before_instruction: vec![],
                    after_instruction: vec![],
                }
            } else {
                InitRelevantInstruction {
                    source: SourceInstruction::Terminator { bb: bb_idx },
                    before_instruction: vec![],
                    after_instruction: vec![],
                }
            };
            if bb_idx == 0 {
                let union_args: Vec<_> = body
                    .locals()
                    .iter()
                    .enumerate()
                    .skip(1)
                    .take(body.arg_count())
                    .filter(|(_, local)| has_union_bytes(local.ty))
                    .collect();
                // Guard shim entry when parameters or the return value may carry
                // memory initialization state, before loading union arguments.
                if self.instance.kind == InstanceKind::Shim
                    && body
                        .locals()
                        .iter()
                        .take(body.arg_count() + 1)
                        .any(|local| shim_type_may_carry_shadow(self.tcx, local.ty))
                {
                    self.push_target(MemoryInitOp::Unsupported {
                        reason: unsupported_call_reason("shadow transfers through Shim instances"),
                    });
                } else if !union_args.is_empty() {
                    for (idx, _) in union_args {
                        self.push_target(MemoryInitOp::LoadArgument {
                            operand: Operand::Copy(Place { local: idx, projection: vec![] }),
                            argument_no: idx,
                        })
                    }
                }
            }
            self.visit_basic_block(bb);
        }
        self.targets
    }
}

impl<'tcx> CheckUninitVisitor<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, instance: Instance) -> Self {
        Self {
            tcx,
            instance,
            locals: vec![],
            box_pointers: HashSet::new(),
            targets: vec![],
            current_target: InitRelevantInstruction {
                source: SourceInstruction::Statement { idx: 0, bb: 0 },
                before_instruction: vec![],
                after_instruction: vec![],
            },
        }
    }

    fn push_target(&mut self, source_op: MemoryInitOp) {
        self.current_target.push_operation(source_op);
    }

    /// Whether `place` is a pointer read out of a `Box`: a field of a `Box`, or a local that only
    /// ever holds one (MIR dereferences `*b` as `*(b.0 as *const T)`, possibly via a temporary).
    fn is_inside_box(&self, place: &Place) -> bool {
        (place.projection.is_empty() && self.box_pointers.contains(&place.local))
            || is_inside_box(place, &self.locals)
    }

    /// Store the state of the call arguments that contain a union in the argument buffer, for the
    /// callee to load (see `LoadArgument`).
    ///
    /// The arguments are numbered as the callee's body numbers its parameters. A call through the
    /// `rust-call` ABI (e.g. `Fn::call(&c, (u, x))`, resolved to the closure) passes the last
    /// arguments as one tuple, which a closure body receives as separate parameters: each field of
    /// that tuple is then its own argument.
    fn store_union_arguments(&mut self, callee: &Instance, args: &[Operand]) {
        if !args.iter().any(|arg| has_union_bytes(arg.ty(&self.locals).unwrap())) {
            return;
        }
        let untupled = args.last().and_then(|last| {
            let RigidTy::Tuple(fields) = last.ty(&self.locals).unwrap().kind().rigid()?.clone()
            else {
                return None;
            };
            // The callee's parameters after the ordinary arguments are the tuple's fields.
            let body = callee.body()?;
            let params: Vec<Ty> = body.arg_locals().iter().map(|local| local.ty).collect();
            let rest = params.get(args.len() - 1..)?;
            (body.spread_arg().is_none() && rest == fields.as_slice())
                .then_some((last.clone(), fields))
        });
        let ordinary = if untupled.is_some() { &args[..args.len() - 1] } else { args };
        for (idx, operand) in ordinary.iter().enumerate() {
            if has_union_bytes(operand.ty(&self.locals).unwrap()) {
                self.store_union_argument(operand.clone(), idx + 1); // arguments are 1-indexed
            }
        }
        let Some((tuple, fields)) = untupled else { return };
        let first = ordinary.len() + 1;
        let place = match &tuple {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            Operand::Constant(_) | Operand::RuntimeChecks(_) => {
                // The fields of a constant tuple have no place of their own either.
                self.store_union_argument(tuple.clone(), first);
                return;
            }
        };
        for (idx, field_ty) in fields.into_iter().enumerate() {
            if has_union_bytes(field_ty) {
                let mut field = place.clone();
                field.projection.push(ProjectionElem::Field(idx, field_ty));
                self.store_union_argument(Operand::Copy(field), first + idx);
            }
        }
    }

    /// Store the state of `operand`, which contains a union, as the callee's argument
    /// `argument_no`.
    fn store_union_argument(&mut self, operand: Operand, argument_no: usize) {
        if let Operand::Constant(_) = operand {
            // A constant has no place to point the argument buffer at. The callee marks an
            // argument it finds no state for as initialized, which is right exactly when every
            // byte of the constant is.
            if !constant_init_mask(&operand).is_some_and(|mask| mask.iter().all(|byte| *byte)) {
                self.push_target(MemoryInitOp::Unsupported {
                    reason: "Kani does not support passing a constant with uninitialized \
                             bytes in a union to a function."
                        .to_string(),
                });
            }
            return;
        }
        self.push_target(MemoryInitOp::StoreArgument { operand, argument_no })
    }

    /// Give `*dst` the state of the value `src`, whose type contains a union.
    fn store_union_value(&mut self, dst: &Operand, src: &Operand) {
        let (Operand::Copy(dst_ptr) | Operand::Move(dst_ptr)) = dst else {
            self.push_target(MemoryInitOp::Unsupported {
                reason: "Kani does not support storing a union through a constant pointer."
                    .to_string(),
            });
            return;
        };
        let mut lvalue = dst_ptr.clone();
        lvalue.projection.push(ProjectionElem::Deref);
        match src {
            Operand::Constant(_) => self.set_from_constant(&lvalue, src),
            _ => self.push_target(MemoryInitOp::AssignUnion { lvalue, rvalue: src.clone() }),
        }
    }

    /// The operation that gives the place `lvalue` the state of the constant `operand`: its
    /// bytes' state is known, or the assignment is unsupported.
    fn set_from_constant(&mut self, lvalue: &Place, operand: &Operand) {
        match constant_init_mask(operand) {
            Some(mask) => self.push_target(MemoryInitOp::SetMasked {
                operand: Operand::Copy(lvalue.clone()),
                mask,
            }),
            None => self.push_target(MemoryInitOp::Unsupported {
                reason: "Kani does not support assigning this constant containing a union."
                    .to_string(),
            }),
        }
    }
}

impl CheckUninitVisitor<'_> {
    /// For an aggregate `place = kind(operands)`, copy the state of each operand that is a place
    /// containing a union to the field of `place` it initializes.
    fn assign_union_fields(&mut self, place: &Place, kind: &AggregateKind, operands: &[Operand]) {
        let num_operands = operands.len() as u64;
        for (idx, operand) in operands.iter().enumerate() {
            let (Operand::Copy(_) | Operand::Move(_)) = operand else { continue };
            let operand_ty = operand.ty(&self.locals).unwrap();
            if !has_union_bytes(operand_ty) {
                continue;
            }
            let elem = match kind {
                AggregateKind::Adt(adt_def, _, _, _, None) if adt_def.kind() == AdtKind::Struct => {
                    ProjectionElem::Field(idx, operand_ty)
                }
                AggregateKind::Tuple => ProjectionElem::Field(idx, operand_ty),
                AggregateKind::Array(_) => ProjectionElem::ConstantIndex {
                    offset: idx as u64,
                    min_length: num_operands,
                    from_end: false,
                },
                // Enum variants and closures keep their union bytes uninitialized, which can
                // only cause false alarms.
                _ => continue,
            };
            let mut field = place.clone();
            field.projection.push(elem);
            self.push_target(MemoryInitOp::AssignUnion { lvalue: field, rvalue: operand.clone() });
        }
    }
}

/// Which bytes of the constant `operand` are initialized, if it is a constant whose bytes are
/// known.
fn constant_init_mask(operand: &Operand) -> Option<Vec<bool>> {
    let Operand::Constant(constant) = operand else { return None };
    match constant.const_.kind() {
        ConstantKind::Allocated(allocation) => {
            Some(allocation.bytes.iter().map(Option::is_some).collect())
        }
        ConstantKind::ZeroSized => Some(vec![]),
        _ => None,
    }
}

/// Whether `ty` contains a reference, e.g. `&T` or a struct with a `&T` field.
fn contains_reference(ty: Ty) -> bool {
    contains_ty(ty, |kind| matches!(kind, RigidTy::Ref(..)))
}

/// Whether `ty` contains a raw pointer, e.g. a union with a `*const T` field.
fn contains_raw_pointer(ty: Ty) -> bool {
    contains_ty(ty, |kind| matches!(kind, RigidTy::RawPtr(..)))
}

/// Whether `ty` or a type nested in it (an element or a field, for every variant of an enum and
/// every field of a union) satisfies `is_match`. Deeply nested or recursive types count as a
/// match, which is the conservative answer for both callers.
fn contains_ty(ty: Ty, is_match: impl Fn(&RigidTy) -> bool + Copy) -> bool {
    fn walk(ty: Ty, depth: usize, is_match: impl Fn(&RigidTy) -> bool + Copy) -> bool {
        if depth > 16 {
            return true;
        }
        let Some(kind) = ty.kind().rigid().cloned() else { return false };
        if is_match(&kind) {
            return true;
        }
        match kind {
            RigidTy::Array(elem, _) | RigidTy::Slice(elem) => walk(elem, depth + 1, is_match),
            RigidTy::Tuple(elems) => elems.iter().any(|elem| walk(*elem, depth + 1, is_match)),
            RigidTy::Adt(def, args) => def.variants_iter().any(|variant| {
                variant
                    .fields()
                    .iter()
                    .any(|field| walk(field.ty_with_args(&args), depth + 1, is_match))
            }),
            _ => false,
        }
    }
    walk(ty, 0, is_match)
}

/// Whether `ty` is a union or contains one (see `UnionMask`).
fn has_union_bytes(ty: Ty) -> bool {
    ty.kind().is_union() || has_nested_union(ty)
}

/// Whether `ty` is not a union but contains one, e.g. `[MaybeUninit<T>; N]`.
fn has_nested_union(ty: Ty) -> bool {
    matches!(
        PointeeInfo::from_ty(ty).map(|info| match info.layout() {
            PointeeLayout::Sized { union_mask, .. } => !union_mask.is_empty(),
            _ => false,
        }),
        Ok(true)
    )
}

impl MirVisitor for CheckUninitVisitor<'_> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        // Leave it as an exhaustive match to be notified when a new kind is added.
        match &stmt.kind {
            StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(copy)) => {
                self.super_statement(stmt, location);
                // The copy is untyped, so we should copy memory initialization state from `src`
                // to `dst`.
                self.push_target(MemoryInitOp::Copy {
                    from: copy.src.clone(),
                    to: copy.dst.clone(),
                    count: copy.count.clone(),
                });
            }
            StatementKind::Assign(place, rvalue) => {
                // First check rvalue.
                self.visit_rvalue(rvalue, location);

                // Reading the metadata of a pointer to a trait object gives access to its vtable,
                // e.g. to read its size and alignment. The vtable is constant data that the code
                // generator emits, so mark its header initialized.
                if let Rvalue::UnaryOp(rustc_public::mir::UnOp::PtrMetadata, operand) = rvalue
                    && let Some(pointee) =
                        operand.ty(&self.locals).unwrap().kind().builtin_deref(true)
                    && pointee.ty.kind().is_trait()
                {
                    self.push_target(MemoryInitOp::SetVtable {
                        metadata: Operand::Copy(place.clone()),
                    });
                }

                // A write of a whole value whose type contains unions (e.g. an array of
                // `MaybeUninit<T>`): the union bytes take the source's state, which the type does not
                // determine. Copy it for a copy or move of another place; otherwise, e.g. for
                // `[MaybeUninit::uninit(); N]`, mark them uninitialized, which can only cause false
                // alarms. The data and padding bytes are then marked as for any write: through a
                // pointer below, or here for other places; those writes leave the union bytes
                // unchanged, so their order relative to the copies does not matter.
                let place_ty = place.ty(&self.locals).unwrap();
                if !place_ty.kind().is_union() && has_nested_union(place_ty) {
                    match rvalue {
                        Rvalue::Use(operand @ (Operand::Copy(_) | Operand::Move(_)), _) => self
                            .push_target(MemoryInitOp::AssignUnion {
                                lvalue: place.clone(),
                                rvalue: operand.clone(),
                            }),
                        Rvalue::Use(operand @ Operand::Constant(_), _) => {
                            self.set_from_constant(place, operand)
                        }
                        _ => {
                            // Reset before the write: operations inserted after an instruction do
                            // not run in the order they are pushed, and the copies below must
                            // come after the reset. The rvalue does not read union state from
                            // the place it overwrites, except in an aggregate that reuses it,
                            // which can only cause false alarms.
                            self.push_target(MemoryInitOp::SetRef {
                                operand: Operand::Copy(place.clone()),
                                value: false,
                                position: InsertPosition::Before,
                            });
                            // A struct, tuple or array built from values that contain unions,
                            // e.g. `S { u }`: each such field takes the state of its operand.
                            if let Rvalue::Aggregate(kind, operands) = rvalue {
                                self.assign_union_fields(place, kind, operands);
                            }
                        }
                    }
                    if try_remove_topmost_deref(place).is_none() {
                        self.push_target(MemoryInitOp::SetRef {
                            operand: Operand::Copy(place.clone()),
                            value: true,
                            position: InsertPosition::After,
                        });
                    }
                }

                // Check whether we are assigning into a dereference (*ptr = _).
                if let Some(place_without_deref) = try_remove_topmost_deref(place) {
                    // First, check that we are not dereferencing extra pointers along the way
                    // (e.g., **ptr = _). If yes, check whether these pointers are initialized.
                    let mut place_to_add_projections =
                        Place { local: place_without_deref.local, projection: vec![] };
                    for projection_elem in place_without_deref.projection.iter() {
                        // If the projection is Deref and the current type is raw pointer, check
                        // if it points to initialized memory.
                        if *projection_elem == ProjectionElem::Deref
                            && let TyKind::RigidTy(RigidTy::RawPtr(..)) =
                                place_to_add_projections.ty(&self.locals).unwrap().kind()
                        {
                            self.push_target(MemoryInitOp::Check {
                                operand: Operand::Copy(place_to_add_projections.clone()),
                            });
                        };
                        place_to_add_projections.projection.push(projection_elem.clone());
                    }
                    if place_without_deref.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                        self.push_target(MemoryInitOp::Set {
                            operand: Operand::Copy(place_without_deref),
                            value: true,
                            position: InsertPosition::After,
                        });
                    }
                }
                // Check whether Rvalue creates a new initialized pointer previously not captured inside shadow memory.
                if place.ty(&self.locals).unwrap().kind().is_raw_ptr()
                    && let Rvalue::AddressOf(..) = rvalue
                {
                    self.push_target(MemoryInitOp::Set {
                        operand: Operand::Copy(place.clone()),
                        value: true,
                        position: InsertPosition::After,
                    });
                }

                // Whether the place, or a place it projects from, is a union, e.g. `u`, `*p` for
                // `p: *mut U`, or `u.field`.
                let is_inside_union = {
                    let mut prefix = Place { local: place.local, projection: vec![] };
                    let mut contains_union = prefix.ty(&self.locals).unwrap().kind().is_union();
                    for projection_elem in place.projection.iter() {
                        if contains_union {
                            break;
                        }
                        prefix.projection.push(projection_elem.clone());
                        contains_union = prefix.ty(&self.locals).unwrap().kind().is_union();
                    }
                    contains_union
                };

                // Need to copy some information about union initialization, since lvalue is
                // either a union or a field inside a union.
                if is_inside_union {
                    match rvalue {
                        Rvalue::Use(operand, _) => {
                            // This is a union-to-union assignment, so we need to copy the
                            // initialization state.
                            if place.ty(&self.locals).unwrap().kind().is_union() {
                                if let Operand::Constant(_) = operand {
                                    self.set_from_constant(place, operand);
                                } else {
                                    self.push_target(MemoryInitOp::AssignUnion {
                                        lvalue: place.clone(),
                                        rvalue: operand.clone(),
                                    });
                                }
                            } else {
                                // This is assignment to a field of a union.
                                self.push_target(MemoryInitOp::SetRef {
                                    operand: Operand::Copy(place.clone()),
                                    value: true,
                                    position: InsertPosition::After,
                                });
                            }
                        }
                        Rvalue::Aggregate(AggregateKind::Adt(adt_def, _, _, _, union_field), _) => {
                            // Create a union from scratch as an aggregate. We handle it here because we
                            // need to know which field is getting assigned.
                            if adt_def.kind() == AdtKind::Union {
                                self.push_target(MemoryInitOp::CreateUnion {
                                    operand: Operand::Copy(place.clone()),
                                    field: union_field.unwrap(), // Safe to unwrap because we know this is a union.
                                });
                            }
                        }
                        // TODO: add support for Rvalue::Cast, etc.
                        _ => self
                            .push_target(MemoryInitOp::Unsupported { reason: "Performing a union assignment with a non-supported construct as an Rvalue".to_string() }),
                    }
                }
            }
            StatementKind::FakeRead(_, _)
            | StatementKind::SetDiscriminant { .. }
            | StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::PlaceMention(_)
            | StatementKind::AscribeUserType { .. }
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::Intrinsic(NonDivergingIntrinsic::Assume(_))
            | StatementKind::Nop => self.super_statement(stmt, location),
        }
        // Switch to the next statement.
        if let SourceInstruction::Statement { idx, bb } = self.current_target.source {
            self.targets.push(self.current_target.clone());
            self.current_target = InitRelevantInstruction {
                source: SourceInstruction::Statement { idx: idx + 1, bb },
                after_instruction: vec![],
                before_instruction: vec![],
            };
        } else {
            unreachable!()
        }
    }

    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if let SourceInstruction::Statement { bb, .. } = self.current_target.source {
            // We don't have to push the previous target, since it already happened in the statement
            // handling code.
            self.current_target = InitRelevantInstruction {
                source: SourceInstruction::Terminator { bb },
                after_instruction: vec![],
                before_instruction: vec![],
            };
        } else {
            // The only instruction in this basic block is the terminator, which was already set.
        }
        // Leave it as an exhaustive match to be notified when a new kind is added.
        match &term.kind {
            TerminatorKind::Call { func, args, destination, .. } => 'call: {
                self.super_terminator(term, location);
                let instance = match try_resolve_instance(self.tcx, &self.locals, func) {
                    Ok(instance) => instance,
                    Err(reason) => {
                        self.push_target(MemoryInitOp::Unsupported { reason });
                        // Publish entry loads and operand checks through the terminator epilogue.
                        break 'call;
                    }
                };
                // Check arguments and the destination for memory initialization state,
                // including unions inside tuples, before storing argument state.
                if instance.kind == InstanceKind::Shim
                    && (args.iter().any(|arg| {
                        shim_type_may_carry_shadow(self.tcx, arg.ty(&self.locals).unwrap())
                    }) || shim_type_may_carry_shadow(
                        self.tcx,
                        destination.ty(&self.locals).unwrap(),
                    ))
                {
                    self.push_target(MemoryInitOp::Unsupported {
                        reason: unsupported_call_reason("shadow transfers through Shim instances"),
                    });
                    break 'call;
                }
                match instance.kind {
                    InstanceKind::Intrinsic => {
                        match Intrinsic::from_instance(&instance) {
                            intrinsic_name if can_skip_intrinsic(intrinsic_name.clone()) => {
                                /* Intrinsics that can be safely skipped */
                            }
                            Intrinsic::AtomicAnd
                            | Intrinsic::AtomicCxchg
                            | Intrinsic::AtomicCxchgWeak
                            | Intrinsic::AtomicLoad
                            | Intrinsic::AtomicMax
                            | Intrinsic::AtomicMin
                            | Intrinsic::AtomicNand
                            | Intrinsic::AtomicOr
                            | Intrinsic::AtomicUmax
                            | Intrinsic::AtomicUmin
                            | Intrinsic::AtomicXadd
                            | Intrinsic::AtomicXchg
                            | Intrinsic::AtomicXor
                            | Intrinsic::AtomicXsub => {
                                self.push_target(MemoryInitOp::Check { operand: args[0].clone() });
                            }
                            Intrinsic::AtomicStore => {
                                let is_volatile = matches!(
                                    instance.args().0[2].expect_const().kind(),
                                    TyConstKind::Value(_, allocation)
                                        if allocation.read_bool().is_ok_and(|value| value)
                                );
                                // Preserve the existing check for non-volatile stores, which are
                                // specified in terms of creating a reference with `from_ptr`.
                                if !is_volatile {
                                    self.push_target(MemoryInitOp::Check {
                                        operand: args[0].clone(),
                                    });
                                }
                                self.push_target(MemoryInitOp::Set {
                                    operand: args[0].clone(),
                                    value: true,
                                    position: InsertPosition::After,
                                });
                            }
                            Intrinsic::CompareBytes => {
                                self.push_target(MemoryInitOp::CheckSliceChunk {
                                    operand: args[0].clone(),
                                    count: args[2].clone(),
                                });
                                self.push_target(MemoryInitOp::CheckSliceChunk {
                                    operand: args[1].clone(),
                                    count: args[2].clone(),
                                });
                            }
                            Intrinsic::Copy => {
                                // The copy is untyped, so we should copy memory
                                // initialization state from `src` to `dst`.
                                self.push_target(MemoryInitOp::Copy {
                                    from: args[0].clone(),
                                    to: args[1].clone(),
                                    count: args[2].clone(),
                                });
                            }
                            Intrinsic::VolatileCopyMemory
                            | Intrinsic::VolatileCopyNonOverlappingMemory => {
                                // The copy is untyped, so we should copy initialization state
                                // from `src` to `dst`. Note that the `dst` comes before `src`
                                // in this case.
                                self.push_target(MemoryInitOp::Copy {
                                    from: args[1].clone(),
                                    to: args[0].clone(),
                                    count: args[2].clone(),
                                });
                            }
                            Intrinsic::TypedSwap => {
                                self.push_target(MemoryInitOp::Check { operand: args[0].clone() });
                                self.push_target(MemoryInitOp::Check { operand: args[1].clone() });
                            }
                            Intrinsic::VolatileLoad | Intrinsic::UnalignedVolatileLoad => {
                                self.push_target(MemoryInitOp::Check { operand: args[0].clone() });
                            }
                            Intrinsic::VolatileStore | Intrinsic::UnalignedVolatileStore => {
                                self.push_target(MemoryInitOp::Set {
                                    operand: args[0].clone(),
                                    value: true,
                                    position: InsertPosition::After,
                                });
                                // The stored value's union bytes keep the value's state, as for
                                // an assignment `*dst = src`: marking them initialized would hide
                                // a store of `MaybeUninit::uninit()`.
                                if let Some(pointee) = args[1].ty(&self.locals).ok()
                                    && has_union_bytes(pointee)
                                {
                                    self.store_union_value(&args[0], &args[1]);
                                }
                            }
                            // `volatile_set_memory(dst, val, count)` has the same shape and the
                            // same memory-initialization effect as `write_bytes(dst, val, count)`;
                            // volatility does not change which bytes become initialized.
                            Intrinsic::WriteBytes | Intrinsic::VolatileSetMemory => self
                                .push_target(MemoryInitOp::SetSliceChunk {
                                    operand: args[0].clone(),
                                    count: args[2].clone(),
                                    value: true,
                                    position: InsertPosition::After,
                                }),
                            // An intrinsic that Kani does not know, but whose fallback body it uses
                            // (e.g. `integer_max`, new in nightly-2026-11-17), runs ordinary MIR,
                            // which is instrumented like any other function: an ordinary call.
                            Intrinsic::Unimplemented { .. }
                                if !crate::intrinsics::uses_kani_model(&instance) => {}
                            intrinsic => {
                                self.push_target(MemoryInitOp::Unsupported {
                                    reason: format!("Kani does not support reasoning about memory initialization of intrinsic `{intrinsic:?}`."),
                                });
                            }
                        }
                    }
                    InstanceKind::Item | InstanceKind::Shim
                        if !instance.is_foreign_item()
                            && has_union_bytes(destination.ty(&self.locals).unwrap()) =>
                    {
                        // The callee stores the state of a returned value containing a union in
                        // the argument buffer before it returns; copy it to the destination. Then
                        // mark the destination's data and padding bytes as for any write.
                        self.store_union_arguments(&instance, args);
                        self.push_target(MemoryInitOp::LoadReturnValue {
                            operand: Operand::Copy(destination.clone()),
                        });
                        if !destination.ty(&self.locals).unwrap().kind().is_union() {
                            self.push_target(MemoryInitOp::SetRef {
                                operand: Operand::Copy(destination.clone()),
                                value: true,
                                position: InsertPosition::After,
                            });
                        }
                    }
                    InstanceKind::Item | InstanceKind::Shim => {
                        // Model foreign allocation calls here. Calls with MIR bodies store
                        // initialization state for union arguments.
                        if instance.is_foreign_item() {
                            match instance.name().as_str() {
                                "alloc::alloc::__rust_alloc" | "alloc::alloc::__rust_realloc" => {
                                    /* Memory is uninitialized, nothing to do here. */
                                }
                                "alloc::alloc::__rust_alloc_zeroed" => {
                                    /* Memory is initialized here, need to update shadow memory. */
                                    self.push_target(MemoryInitOp::SetSliceChunk {
                                        operand: Operand::Copy(destination.clone()),
                                        count: args[0].clone(),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
                                "alloc::alloc::__rust_dealloc" => {
                                    /* Memory is uninitialized here, need to update shadow memory. */
                                    self.push_target(MemoryInitOp::SetSliceChunk {
                                        operand: raw_ptr_arg(&args[0], self.locals.as_slice()),
                                        count: args[1].clone(),
                                        value: false,
                                        position: InsertPosition::After,
                                    });
                                }
                                _ => {}
                            }
                        } else {
                            self.store_union_arguments(&instance, args);
                        }
                    }
                    InstanceKind::Virtual { .. } | InstanceKind::LlvmIntrinsic => {
                        unreachable!("the shared resolver rejects unsupported callees")
                    }
                }
            }
            TerminatorKind::Drop { place, .. } => {
                self.super_terminator(term, location);
                let place_ty = place.ty(&self.locals).unwrap();

                // When drop is codegen'ed for types that could define their own dropping
                // behavior, a reference is taken to the place which is later implicitly coerced
                // to a pointer. Hence, we need to bless this pointer as initialized.
                match place
                    .ty(&self.locals)
                    .unwrap()
                    .kind()
                    .rigid()
                    .expect("should be working with monomorphized code")
                {
                    RigidTy::Adt(..) | RigidTy::Dynamic(_, _) => {
                        self.push_target(MemoryInitOp::SetRef {
                            operand: Operand::Copy(place.clone()),
                            value: true,
                            position: InsertPosition::Before,
                        });
                    }
                    _ => {}
                }

                if place_ty.kind().is_raw_ptr() {
                    self.push_target(MemoryInitOp::Set {
                        operand: Operand::Copy(place.clone()),
                        value: false,
                        position: InsertPosition::After,
                    });
                }
            }
            TerminatorKind::Return if has_union_bytes(self.locals[0].ty) => {
                // The caller loads the state of a returned value containing a union from the
                // argument buffer after the call (see `LoadReturnValue`).
                self.super_terminator(term, location);
                self.push_target(MemoryInitOp::StoreArgument {
                    operand: Operand::Copy(Place::from(0)),
                    argument_no: 0,
                });
            }
            TerminatorKind::Goto { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Abort
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::Assert { .. }
            | TerminatorKind::InlineAsm { .. } => self.super_terminator(term, location),
        }
        // Push the current target from the terminator onto the list.
        self.targets.push(self.current_target.clone());
    }

    fn visit_place(&mut self, place: &Place, ptx: PlaceContext, location: Location) {
        for (idx, elem) in place.projection.iter().enumerate() {
            let intermediate_place =
                Place { local: place.local, projection: place.projection[..idx].to_vec() };
            match elem {
                ProjectionElem::Deref => {
                    let ptr_ty = intermediate_place.ty(&self.locals).unwrap();
                    if ptr_ty.kind().is_raw_ptr() && !points_to_trait_object(ptr_ty) {
                        self.push_target(MemoryInitOp::Check {
                            operand: Operand::Copy(intermediate_place.clone()),
                        });
                    }
                    // A place `*p` of a trait object type is never read: it is unsized, so it can
                    // only be borrowed or have its address taken. Taking its address reads
                    // nothing. Borrowing it is handled in `visit_rvalue`.
                }
                ProjectionElem::Field(_, _) => {
                    if intermediate_place.ty(&self.locals).unwrap().kind().is_union()
                        && !ptx.is_mutating()
                    {
                        // Accessing a place inside the union, need to check if it is initialized.
                        // This works the same through a pointer (`(*ptr).field`): the check is on
                        // the field's bytes at its address.
                        self.push_target(MemoryInitOp::CheckRef {
                            operand: Operand::Copy(place.clone()),
                        });
                    }
                }
                ProjectionElem::Index(_)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Subslice { .. } => {
                    /* For a slice to be indexed, it should be valid first. */
                }
                ProjectionElem::Downcast(_) => {}
                ProjectionElem::OpaqueCast(_) => {}
            }
        }
        self.super_place(place, ptx, location)
    }

    fn visit_operand(&mut self, operand: &Operand, location: Location) {
        if let Operand::Constant(constant) = operand
            && let ConstantKind::Allocated(allocation) = constant.const_.kind()
        {
            for (_, prov) in &allocation.provenance.ptrs {
                if let GlobalAlloc::Static(_) = GlobalAlloc::from(prov.0)
                    && constant.ty().kind().is_raw_ptr()
                {
                    // If a static is a raw pointer, need to mark it as initialized.
                    self.push_target(MemoryInitOp::Set {
                        operand: Operand::Constant(constant.clone()),
                        value: true,
                        position: InsertPosition::Before,
                    });
                };
            }
        }
        self.super_operand(operand, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        // A reference must point to a valid value, but the layout of the value behind a raw
        // pointer to a trait object is only known from its vtable at run time, so its
        // initialization cannot be checked here. Neither can the concrete method a call through
        // the reference reaches, since Kani does not check reads through references.
        // A `Box`, like a reference, points to a valid value; MIR dereferences it through the
        // raw pointer inside it, so a raw pointer reached through a `Box` is exempt.
        if let Rvalue::Ref(_, _, place) = rvalue
            && let Some(ptr) = try_remove_topmost_deref(place)
            && ptr.ty(&self.locals).unwrap().kind().is_raw_ptr()
            && points_to_trait_object(ptr.ty(&self.locals).unwrap())
            && !self.is_inside_box(&ptr)
        {
            self.push_target(MemoryInitOp::Unsupported {
                reason: "Kani does not support checking the initialization of a reference to a \
                         trait object created from a raw pointer."
                    .to_string(),
            });
        }
        if let Rvalue::Cast(cast_kind, operand, ty) = rvalue {
            match cast_kind {
                CastKind::PointerCoercion(PointerCoercion::Unsize) => {
                    // Unsizing a reference to a sized value into a raw pointer to a trait object
                    // creates a raw pointer to that value, whose type is still known here. Mark it
                    // initialized with that type's layout, as for any raw pointer created from a
                    // reference: a reference points to a valid value. Unsizing a raw pointer
                    // creates no new pointee and must keep the pointee's state, initialized or
                    // not; so does unsizing an already unsized pointer (e.g. trait upcasting).
                    if let TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) = ty.kind()
                        && pointee_ty.kind().is_trait()
                        && let TyKind::RigidTy(RigidTy::Ref(_, source_ty, _)) =
                            operand.ty(&self.locals).unwrap().kind()
                        && !source_ty.kind().is_trait()
                    {
                        self.push_target(MemoryInitOp::SetAs {
                            operand: operand.clone(),
                            pointee_ty: source_ty,
                            value: true,
                            position: InsertPosition::After,
                        });
                    }
                }
                CastKind::Transmute => {
                    let operand_ty = operand.ty(&self.locals).unwrap();
                    let requirement = transmute_requirement(&operand_ty, ty);
                    if let (
                        TransmuteRequirement::Dynamic(mask),
                        Operand::Copy(place) | Operand::Move(place),
                    ) = (&requirement, operand)
                    {
                        // Bytes of a union in the source become data bytes of the target
                        // (`MaybeUninit::assume_init`): whether they are initialized depends on
                        // what was written to the union, so check its memory.
                        self.push_target(MemoryInitOp::CheckMasked {
                            operand: Operand::Copy(place.clone()),
                            mask: mask.clone(),
                        });
                        // A reference must also point to a valid value, but if the union can
                        // hold a raw pointer, it may point to uninitialized memory. Reads through
                        // references are not checked, so check the pointee here. A union holding
                        // only references (`MaybeUninit<&T>`) needs no check: a reference was
                        // valid when it was written, and Kani does not track the memory behind
                        // references, so checking it would be a false alarm.
                        if contains_raw_pointer(operand_ty) {
                            match ty.kind().rigid() {
                                Some(RigidTy::Ref(_, pointee, _))
                                    if PointeeInfo::from_ty(*pointee).is_ok_and(|info| {
                                        matches!(
                                            info.layout(),
                                            PointeeLayout::Sized { .. }
                                                | PointeeLayout::Union { .. }
                                        )
                                    }) =>
                                {
                                    self.push_target(MemoryInitOp::CheckAs {
                                        operand: operand.clone(),
                                        pointee_ty: *pointee,
                                    })
                                }
                                _ if contains_reference(*ty) => {
                                    self.push_target(MemoryInitOp::Unsupported {
                                        reason:
                                            "Kani does not support checking the value behind a \
                                             reference transmuted from a union."
                                                .to_string(),
                                    })
                                }
                                _ => {}
                            }
                        }
                    } else if !matches!(requirement, TransmuteRequirement::Compatible)
                        && !tys_layout_compatible_to_size(&operand_ty, ty)
                    {
                        // If transmuting between two types of incompatible layouts, padding
                        // bytes are exposed, which is UB.
                        self.push_target(MemoryInitOp::TriviallyUnsafe {
                            reason: "Transmuting between types of incompatible layouts."
                                .to_string(),
                        });
                    } else if let (
                        TyKind::RigidTy(RigidTy::Ref(_, from_ty, _)),
                        TyKind::RigidTy(RigidTy::Ref(_, to_ty, _)),
                    ) = (operand_ty.kind(), ty.kind())
                    {
                        if !tys_layout_compatible_to_size(&from_ty, &to_ty) {
                            // Since references are supposed to always be initialized for its type,
                            // transmuting between two references of incompatible layout is UB.
                            self.push_target(MemoryInitOp::TriviallyUnsafe {
                                reason: "Transmuting between references pointing to types of incompatible layouts."
                                    .to_string(),
                            });
                        }
                    } else if let (
                        TyKind::RigidTy(RigidTy::RawPtr(..)),
                        TyKind::RigidTy(RigidTy::Ref(_, to_ty, _)),
                    ) = (operand_ty.kind(), ty.kind())
                    {
                        // When transmuting from a raw pointer to a reference, need to check that
                        // the value the reference points to is initialized. That is a `to_ty`,
                        // whatever the pointer pointed to before.
                        self.push_target(MemoryInitOp::CheckAs {
                            operand: operand.clone(),
                            pointee_ty: to_ty,
                        });
                    }
                }
                _ => {}
            }
        };
        self.super_rvalue(rvalue, location);
    }
}

/// Determines if the intrinsic has no memory initialization related function and hence can be
/// safely skipped.
fn can_skip_intrinsic(intrinsic: Intrinsic) -> bool {
    match intrinsic {
        Intrinsic::AddWithOverflow
        | Intrinsic::AlignOfVal
        | Intrinsic::ArithOffset
        | Intrinsic::AssertInhabited
        | Intrinsic::AssertMemUninitializedValid
        | Intrinsic::AssertZeroValid
        | Intrinsic::Assume
        | Intrinsic::Bitreverse
        | Intrinsic::BlackBox
        | Intrinsic::Breakpoint
        | Intrinsic::Bswap
        | Intrinsic::CeilF32
        | Intrinsic::CeilF64
        | Intrinsic::CopySignF32
        | Intrinsic::CopySignF64
        | Intrinsic::CosF32
        | Intrinsic::CosF64
        | Intrinsic::Ctlz
        | Intrinsic::CtlzNonZero
        | Intrinsic::Ctpop
        | Intrinsic::Cttz
        | Intrinsic::CttzNonZero
        | Intrinsic::DiscriminantValue
        | Intrinsic::ExactDiv
        | Intrinsic::Exp2F32
        | Intrinsic::Exp2F64
        | Intrinsic::ExpF32
        | Intrinsic::ExpF64
        | Intrinsic::FabsF128
        | Intrinsic::FabsF16
        | Intrinsic::FabsF32
        | Intrinsic::FabsF64
        | Intrinsic::FaddFast
        | Intrinsic::FdivFast
        | Intrinsic::FloorF32
        | Intrinsic::FloorF64
        | Intrinsic::FmafF32
        | Intrinsic::FmafF64
        | Intrinsic::FmulFast
        | Intrinsic::Forget
        | Intrinsic::FsubFast
        | Intrinsic::IsValStaticallyKnown
        | Intrinsic::Likely
        | Intrinsic::Log10F32
        | Intrinsic::Log10F64
        | Intrinsic::Log2F32
        | Intrinsic::Log2F64
        | Intrinsic::LogF32
        | Intrinsic::LogF64
        | Intrinsic::MaxNumF32
        | Intrinsic::MaxNumF64
        | Intrinsic::MinNumF32
        | Intrinsic::MinNumF64
        | Intrinsic::MulWithOverflow
        | Intrinsic::PowF32
        | Intrinsic::PowF64
        | Intrinsic::PowIF32
        | Intrinsic::PowIF64
        | Intrinsic::RawEq
        | Intrinsic::RotateLeft
        | Intrinsic::RotateRight
        | Intrinsic::RoundF32
        | Intrinsic::RoundF64
        | Intrinsic::SaturatingAdd
        | Intrinsic::SaturatingSub
        | Intrinsic::SinF32
        | Intrinsic::SinF64
        | Intrinsic::SqrtF32
        | Intrinsic::SqrtF64
        | Intrinsic::SubWithOverflow
        | Intrinsic::TruncF32
        | Intrinsic::TruncF64
        | Intrinsic::UncheckedDiv
        | Intrinsic::UncheckedRem
        | Intrinsic::Unlikely
        | Intrinsic::VtableSize
        | Intrinsic::VtableAlign
        | Intrinsic::WrappingAdd
        | Intrinsic::WrappingMul
        | Intrinsic::WrappingSub => {
            /* Intrinsics that do not interact with memory initialization. */
            true
        }
        Intrinsic::PtrGuaranteedCmp
        | Intrinsic::PtrOffsetFrom
        | Intrinsic::PtrOffsetFromUnsigned
        | Intrinsic::SizeOfVal => {
            /* AFAICS from the documentation, none of those require the pointer arguments to be actually initialized. */
            true
        }
        Intrinsic::SimdAdd
        | Intrinsic::SimdAnd
        | Intrinsic::SimdDiv
        | Intrinsic::SimdRem
        | Intrinsic::SimdReduceAll
        | Intrinsic::SimdReduceMax
        | Intrinsic::SimdReduceMin
        | Intrinsic::SimdEq
        | Intrinsic::SimdExtract
        | Intrinsic::SimdGe
        | Intrinsic::SimdGt
        | Intrinsic::SimdInsert
        | Intrinsic::SimdLe
        | Intrinsic::SimdLt
        | Intrinsic::SimdMul
        | Intrinsic::SimdNe
        | Intrinsic::SimdOr
        | Intrinsic::SimdShl
        | Intrinsic::SimdShr
        | Intrinsic::SimdShuffle(_)
        | Intrinsic::SimdSplat
        | Intrinsic::SimdSub
        | Intrinsic::SimdXor => {
            /* SIMD operations */
            true
        }
        Intrinsic::AtomicFence | Intrinsic::AtomicSingleThreadFence => {
            /* Atomic fences */
            true
        }
        _ => {
            /* Everything else */
            false
        }
    }
}

/// Try removing a topmost deref projection from a place if it exists, returning a place without it.
/// Whether `place` is a field of a `Box`, e.g. `b.0`.
fn is_inside_box(place: &Place, locals: &[LocalDecl]) -> bool {
    (0..place.projection.len()).any(|len| {
        let prefix = Place { local: place.local, projection: place.projection[..len].to_vec() };
        prefix.ty(locals).unwrap().kind().is_box()
    })
}

/// Locals of `body` that only ever hold a pointer read out of a `Box`: every assignment to them
/// copies or casts a field of a `Box`, and they are never borrowed mutably or written through a
/// projection.
fn box_pointer_locals(body: &MutableBody, locals: &[LocalDecl]) -> HashSet<Local> {
    let mut candidates = HashSet::new();
    let mut disqualified = HashSet::new();
    for bb in body.blocks() {
        for stmt in &bb.statements {
            let StatementKind::Assign(place, rvalue) = &stmt.kind else { continue };
            let from_box = match rvalue {
                Rvalue::Use(Operand::Copy(src) | Operand::Move(src), _)
                | Rvalue::Cast(_, Operand::Copy(src) | Operand::Move(src), _) => {
                    is_inside_box(src, locals)
                }
                _ => false,
            };
            if place.projection.is_empty() && from_box {
                candidates.insert(place.local);
            } else {
                disqualified.insert(place.local);
            }
            if let Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed)
            | Rvalue::AddressOf(RawPtrKind::Mut, borrowed) = rvalue
            {
                disqualified.insert(borrowed.local);
            }
        }
    }
    candidates.retain(|local| !disqualified.contains(local));
    candidates
}

/// Whether `ptr_ty` is a pointer or reference to a trait object.
fn points_to_trait_object(ptr_ty: Ty) -> bool {
    ptr_ty.kind().builtin_deref(true).is_some_and(|pointee| pointee.ty.kind().is_trait())
}

fn try_remove_topmost_deref(place: &Place) -> Option<Place> {
    let mut new_place = place.clone();
    if let Some(ProjectionElem::Deref) = new_place.projection.pop() {
        Some(new_place)
    } else {
        None
    }
}

/// Resolve a call operand and discard the checked body.
fn try_resolve_instance(
    tcx: TyCtxt,
    locals: &[LocalDecl],
    func: &Operand,
) -> Result<Instance, String> {
    resolve_call(tcx, func.ty(locals).unwrap()).map(|(instance, _)| instance)
}

/// Check for unions or unclassified types in a shim's by-value storage.
/// Follow aggregate fields and captures; stop at pointers and references.
fn shim_type_may_carry_shadow(tcx: TyCtxt<'_>, ty: Ty) -> bool {
    fn inspect<'tcx>(
        tcx: TyCtxt<'tcx>,
        ty: rustc_middle::ty::Ty<'tcx>,
        visited: &mut HashSet<rustc_middle::ty::Ty<'tcx>>,
    ) -> bool {
        use rustc_middle::ty::TyKind;
        if !visited.insert(ty) {
            return false;
        }
        match ty.kind() {
            TyKind::Adt(def, args) => {
                def.is_union()
                    || def.all_fields().any(|field| {
                        inspect(tcx, field.ty(tcx, *args).skip_normalization(), visited)
                    })
            }
            TyKind::Tuple(fields) => fields.iter().any(|field| inspect(tcx, field, visited)),
            TyKind::Array(element, _) | TyKind::Pat(element, _) => inspect(tcx, *element, visited),
            TyKind::Closure(_, args) => inspect(tcx, args.as_closure().tupled_upvars_ty(), visited),
            TyKind::Bool
            | TyKind::Char
            | TyKind::Int(_)
            | TyKind::Uint(_)
            | TyKind::Float(_)
            | TyKind::Never
            | TyKind::FnDef(..)
            | TyKind::FnPtr(..)
            | TyKind::RawPtr(..)
            | TyKind::Ref(..) => false,
            _ => true,
        }
    }
    inspect(tcx, rustc_internal::internal(tcx, ty), &mut HashSet::new())
}

fn unsupported_call_reason(kind: &str) -> String {
    format!(
        "Kani does not support memory initialization checks across {kind}. For more information about planned support, see https://github.com/model-checking/kani/issues/3300."
    )
}

/// The pointer argument of an allocation shim, as a raw pointer.
///
/// These shims take `NonNull<u8>` as of nightly-2026-03-21, where they previously took `*mut u8`.
/// `NonNull<T>` holds the address in a single field, which as of nightly-2026-05-01 is a
/// `pattern_type!(*const T is !null)` rather than a bare `*const T`. Project through those
/// single-field wrappers until the raw pointer is reached, so that the shadow-memory models see the
/// same address with the pointer type they expect. Operands that are already raw pointers, and any
/// shape we cannot project all the way down to a pointer, are passed through unchanged.
fn raw_ptr_arg(arg: &Operand, locals: &[LocalDecl]) -> Operand {
    let Ok(arg_ty) = arg.ty(locals) else { return arg.clone() };
    if nonnull_pointee(arg_ty).is_none() {
        return arg.clone();
    }
    let place = match arg {
        Operand::Copy(place) | Operand::Move(place) => place,
        Operand::Constant(_) | Operand::RuntimeChecks(_) => return arg.clone(),
    };
    let mut projection = place.projection.clone();
    let mut curr_ty = arg_ty;
    while !curr_ty.kind().is_raw_ptr() {
        let Some(field_ty) = single_field_ty(curr_ty) else { return arg.clone() };
        projection.push(ProjectionElem::Field(0, field_ty));
        curr_ty = field_ty;
    }
    Operand::Copy(Place { local: place.local, projection })
}

/// The type held by `ty`'s only field, if `ty` is a single-field wrapper.
///
/// This sees through both structs with exactly one field (such as `NonNull`) and pattern types,
/// which hold the type they constrain in a single tuple-like field.
fn single_field_ty(ty: Ty) -> Option<Ty> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Adt(def, args)) => {
            let variant = def.variants_iter().next()?;
            let fields = variant.fields();
            if fields.len() != 1 {
                return None;
            }
            Some(fields[0].ty_with_args(&args))
        }
        TyKind::RigidTy(RigidTy::Pat(base_ty, _)) => Some(base_ty),
        _ => None,
    }
}
