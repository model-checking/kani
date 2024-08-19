// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Visitor that collects all instructions relevant to uninitialized memory access.

use crate::{
    intrinsics::Intrinsic,
    kani_middle::transform::{
        body::{InsertPosition, MutableBody, SourceInstruction},
        check_uninit::{
            relevant_instruction::{InitRelevantInstruction, MemoryInitOp},
            ty_layout::tys_layout_compatible_to_size,
            TargetFinder,
        },
    },
};
use stable_mir::{
    mir::{
        alloc::GlobalAlloc,
        mono::{Instance, InstanceKind},
        BasicBlock, CastKind, CopyNonOverlapping, InlineAsmOperand, LocalDecl,
        NonDivergingIntrinsic, Operand, Place, PointerCoercion, ProjectionElem, Rvalue, Statement,
        StatementKind, Terminator, TerminatorKind,
    },
    ty::{ConstantKind, RigidTy, TyKind},
};

pub struct CheckUninitVisitor {
    locals: Vec<LocalDecl>,
    /// The target instruction that should be verified.
    pub target: Option<InitRelevantInstruction>,
}

impl TargetFinder for CheckUninitVisitor {
    fn find_next(
        &mut self,
        body: &MutableBody,
        source: &SourceInstruction,
    ) -> Option<InitRelevantInstruction> {
        self.locals = body.locals().to_vec();
        self.target = None;
        match *source {
            SourceInstruction::Statement { idx, bb } => {
                let BasicBlock { statements, .. } = &body.blocks()[bb];
                let stmt = &statements[idx];
                self.check_statement(stmt)
            }
            SourceInstruction::Terminator { bb } => {
                let BasicBlock { terminator, .. } = &body.blocks()[bb];
                self.check_terminator(terminator)
            }
        }
        self.target.clone()
    }
}

impl CheckUninitVisitor {
    pub fn new() -> Self {
        Self { locals: vec![], target: None }
    }

    fn push_target(&mut self, source_op: MemoryInitOp) {
        let target = self.target.get_or_insert_with(|| InitRelevantInstruction {
            after_instruction: vec![],
            before_instruction: vec![],
        });
        target.push_operation(source_op);
    }
}

impl CheckUninitVisitor {
    /// Check the statement and find all potential instrumentation targets.
    fn check_statement(&mut self, stmt: &Statement) {
        // Leave it as an exhaustive match to be notified when a new kind is added.
        match &stmt.kind {
            StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(
                CopyNonOverlapping { src, dst, count },
            )) => {
                self.check_operand(src);
                self.check_operand(dst);
                self.check_operand(count);
                // The copy is untyped, so we should copy memory initialization state from `src`
                // to `dst`.
                self.push_target(MemoryInitOp::Copy {
                    from: src.clone(),
                    to: dst.clone(),
                    count: count.clone(),
                });
            }
            StatementKind::Assign(place, rvalue) => {
                // First check rvalue.
                self.check_rvalue(rvalue);
                // Check whether we are assigning into a dereference (*ptr = _).
                if let Some(place_without_deref) = try_remove_topmost_deref(place) {
                    // First, check that we are not dereferencing extra pointers along the way
                    // (e.g., **ptr = _). If yes, check whether these pointers are initialized.
                    let mut place_to_add_projections =
                        Place { local: place_without_deref.local, projection: vec![] };
                    for projection_elem in place_without_deref.projection.iter() {
                        // If the projection is Deref and the current type is raw pointer, check
                        // if it points to initialized memory.
                        if *projection_elem == ProjectionElem::Deref {
                            if let TyKind::RigidTy(RigidTy::RawPtr(_, _)) =
                                place_to_add_projections.ty(&self.locals).unwrap().kind()
                            {
                                self.push_target(MemoryInitOp::Check {
                                    operand: Operand::Copy(place_to_add_projections.clone()),
                                });
                            };
                        }
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
                if place.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                    if let Rvalue::AddressOf(_, _) = rvalue {
                        self.push_target(MemoryInitOp::Set {
                            operand: Operand::Copy(place.clone()),
                            value: true,
                            position: InsertPosition::After,
                        });
                    }
                }
            }
            StatementKind::Deinit(place) => {
                self.check_place(place);
                self.push_target(MemoryInitOp::Set {
                    operand: Operand::Copy(place.clone()),
                    value: false,
                    position: InsertPosition::After,
                });
            }
            StatementKind::FakeRead(_, place)
            | StatementKind::SetDiscriminant { place, .. }
            | StatementKind::Retag(_, place)
            | StatementKind::PlaceMention(place)
            | StatementKind::AscribeUserType { place, .. } => {
                self.check_place(place);
            }
            StatementKind::Intrinsic(NonDivergingIntrinsic::Assume(operand)) => {
                self.check_operand(operand);
            }
            StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop => {}
        }
    }

    /// Check the terminator and find all potential instrumentation targets.
    fn check_terminator(&mut self, term: &Terminator) {
        // Leave it as an exhaustive match to be notified when a new kind is added.
        match &term.kind {
            TerminatorKind::Call { func, args, destination, .. } => {
                self.check_operand(func);
                for arg in args {
                    self.check_operand(arg);
                }
                self.check_place(destination);
                let instance = match try_resolve_instance(&self.locals, func) {
                    Ok(instance) => instance,
                    Err(reason) => {
                        self.push_target(MemoryInitOp::Unsupported { reason });
                        return;
                    }
                };
                match instance.kind {
                    InstanceKind::Intrinsic => {
                        match Intrinsic::from_instance(&instance) {
                            intrinsic_name if can_skip_intrinsic(intrinsic_name.clone()) => {
                                /* Intrinsics that can be safely skipped */
                            }
                            Intrinsic::AtomicAnd(_)
                            | Intrinsic::AtomicCxchg(_)
                            | Intrinsic::AtomicCxchgWeak(_)
                            | Intrinsic::AtomicLoad(_)
                            | Intrinsic::AtomicMax(_)
                            | Intrinsic::AtomicMin(_)
                            | Intrinsic::AtomicNand(_)
                            | Intrinsic::AtomicOr(_)
                            | Intrinsic::AtomicStore(_)
                            | Intrinsic::AtomicUmax(_)
                            | Intrinsic::AtomicUmin(_)
                            | Intrinsic::AtomicXadd(_)
                            | Intrinsic::AtomicXchg(_)
                            | Intrinsic::AtomicXor(_)
                            | Intrinsic::AtomicXsub(_) => {
                                self.push_target(MemoryInitOp::Check { operand: args[0].clone() });
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
                            Intrinsic::VolatileStore => {
                                self.push_target(MemoryInitOp::Set {
                                    operand: args[0].clone(),
                                    value: true,
                                    position: InsertPosition::After,
                                });
                            }
                            Intrinsic::WriteBytes => {
                                self.push_target(MemoryInitOp::SetSliceChunk {
                                    operand: args[0].clone(),
                                    count: args[2].clone(),
                                    value: true,
                                    position: InsertPosition::After,
                                })
                            }
                            intrinsic => {
                                self.push_target(MemoryInitOp::Unsupported {
                                    reason: format!("Kani does not support reasoning about memory initialization of intrinsic `{intrinsic:?}`."),
                                });
                            }
                        }
                    }
                    InstanceKind::Item => {
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
                                        operand: args[0].clone(),
                                        count: args[1].clone(),
                                        value: false,
                                        position: InsertPosition::After,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            TerminatorKind::Drop { place, .. } => {
                self.check_place(place);

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
                    RigidTy::Adt(_, _) | RigidTy::Dynamic(_, _, _) => {
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
            TerminatorKind::Goto { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Abort
            | TerminatorKind::Return
            | TerminatorKind::Unreachable => {}
            TerminatorKind::SwitchInt { discr, .. } => {
                self.check_operand(discr);
            }
            TerminatorKind::Assert { cond, .. } => {
                self.check_operand(cond);
            }
            TerminatorKind::InlineAsm { operands, .. } => {
                for op in operands {
                    let InlineAsmOperand { in_value, out_place, raw_rpr: _ } = op;
                    if let Some(input) = in_value {
                        self.check_operand(input);
                    }
                    if let Some(output) = out_place {
                        self.check_place(output);
                    }
                }
            }
        }
    }

    /// Check the rvalue and find all potential instrumentation targets.
    fn check_rvalue(&mut self, rvalue: &Rvalue) {
        match rvalue {
            Rvalue::Aggregate(_, operands) => {
                for op in operands {
                    self.check_operand(op);
                }
            }
            Rvalue::BinaryOp(_, lhs, rhs) | Rvalue::CheckedBinaryOp(_, lhs, rhs) => {
                self.check_operand(lhs);
                self.check_operand(rhs);
            }
            Rvalue::AddressOf(_, place)
            | Rvalue::CopyForDeref(place)
            | Rvalue::Discriminant(place)
            | Rvalue::Len(place)
            | Rvalue::Ref(_, _, place) => {
                self.check_place(place);
            }
            Rvalue::ShallowInitBox(op, _)
            | Rvalue::UnaryOp(_, op)
            | Rvalue::Use(op)
            | Rvalue::Repeat(op, _) => {
                self.check_operand(op);
            }
            Rvalue::NullaryOp(_, _) | Rvalue::ThreadLocalRef(_) => {}
            Rvalue::Cast(cast_kind, op, ty) => {
                self.check_operand(op);
                match cast_kind {
                    // We currently do not support soundly reasoning about trait objects, so need to
                    // notify the user.
                    CastKind::PointerCoercion(PointerCoercion::Unsize) => {
                        if let TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) = ty.kind() {
                            if pointee_ty.kind().is_trait() {
                                self.push_target(MemoryInitOp::Unsupported {
                                reason: "Kani does not support reasoning about memory initialization of unsized pointers.".to_string(),
                            });
                            }
                        }
                    }
                    CastKind::Transmute => {
                        let operand_ty = op.ty(&self.locals).unwrap();
                        if !tys_layout_compatible_to_size(&operand_ty, &ty) {
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
                            TyKind::RigidTy(RigidTy::RawPtr(from_ty, _)),
                            TyKind::RigidTy(RigidTy::Ref(_, to_ty, _)),
                        ) = (operand_ty.kind(), ty.kind())
                        {
                            // Assert that we can only cast this way if types are the same.
                            assert!(from_ty == to_ty);
                            // When transmuting from a raw pointer to a reference, need to check that
                            // the value pointed by the raw pointer is initialized.
                            self.push_target(MemoryInitOp::Check { operand: op.clone() });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Check if one of the place projections involves dereferencing a raw pointer, which is an
    /// instrumentation target , or union access, which is currently not supported.
    fn check_place(&mut self, place: &Place) {
        for (idx, elem) in place.projection.iter().enumerate() {
            let intermediate_place =
                Place { local: place.local, projection: place.projection[..idx].to_vec() };
            self.check_projection_elem(elem, intermediate_place)
        }
    }

    /// Check if the projection involves dereferencing a raw pointer, which is an instrumentation
    /// target, or union access, which is currently not supported.
    fn check_projection_elem(
        &mut self,
        projection_elem: &ProjectionElem,
        intermediate_place: Place,
    ) {
        match projection_elem {
            ProjectionElem::Deref => {
                let ptr_ty = intermediate_place.ty(&self.locals).unwrap();
                if ptr_ty.kind().is_raw_ptr() {
                    self.push_target(MemoryInitOp::Check {
                        operand: Operand::Copy(intermediate_place.clone()),
                    });
                }
            }
            ProjectionElem::Field(_, _) => {
                if intermediate_place.ty(&self.locals).unwrap().kind().is_union() {
                    self.push_target(MemoryInitOp::Unsupported {
                        reason:
                            "Kani does not support reasoning about memory initialization of unions."
                                .to_string(),
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
            ProjectionElem::Subtype(_) => {}
        }
    }

    /// Check if the operand is a static to initialize it or else check its associated place.
    fn check_operand(&mut self, operand: &Operand) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.check_place(place),
            Operand::Constant(constant) => {
                if let ConstantKind::Allocated(allocation) = constant.const_.kind() {
                    for (_, prov) in &allocation.provenance.ptrs {
                        if let GlobalAlloc::Static(_) = GlobalAlloc::from(prov.0) {
                            if constant.ty().kind().is_raw_ptr() {
                                // If a static is a raw pointer, need to mark it as initialized.
                                self.push_target(MemoryInitOp::Set {
                                    operand: Operand::Constant(constant.clone()),
                                    value: true,
                                    position: InsertPosition::Before,
                                });
                            }
                        };
                    }
                }
            }
        }
    }
}

/// Determines if the intrinsic has no memory initialization related function and hence can be
/// safely skipped.
fn can_skip_intrinsic(intrinsic: Intrinsic) -> bool {
    match intrinsic {
        Intrinsic::AddWithOverflow
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
        | Intrinsic::MinAlignOf
        | Intrinsic::MinAlignOfVal
        | Intrinsic::MinNumF32
        | Intrinsic::MinNumF64
        | Intrinsic::MulWithOverflow
        | Intrinsic::NearbyIntF32
        | Intrinsic::NearbyIntF64
        | Intrinsic::NeedsDrop
        | Intrinsic::PowF32
        | Intrinsic::PowF64
        | Intrinsic::PowIF32
        | Intrinsic::PowIF64
        | Intrinsic::PrefAlignOf
        | Intrinsic::RawEq
        | Intrinsic::RintF32
        | Intrinsic::RintF64
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
        | Intrinsic::TypeId
        | Intrinsic::TypeName
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
        | Intrinsic::SimdSub
        | Intrinsic::SimdXor => {
            /* SIMD operations */
            true
        }
        Intrinsic::AtomicFence(_) | Intrinsic::AtomicSingleThreadFence(_) => {
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
fn try_remove_topmost_deref(place: &Place) -> Option<Place> {
    let mut new_place = place.clone();
    if let Some(ProjectionElem::Deref) = new_place.projection.pop() {
        Some(new_place)
    } else {
        None
    }
}

/// Try retrieving instance for the given function operand.
fn try_resolve_instance(locals: &[LocalDecl], func: &Operand) -> Result<Instance, String> {
    let ty = func.ty(locals).unwrap();
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnDef(def, args)) => Ok(Instance::resolve(def, &args).unwrap()),
        _ => Err(format!(
            "Kani was not able to resolve the instance of the function operand `{ty:?}`. Currently, memory initialization checks in presence of function pointers and vtable calls are not supported. For more information about planned support, see https://github.com/model-checking/kani/issues/3300."
        )),
    }
}
