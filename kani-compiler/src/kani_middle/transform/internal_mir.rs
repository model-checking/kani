// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains conversions between from stable MIR data structures to its internal
//! counterparts. This is primarily done to facilitate using dataflow analysis, which does not yet
//! support StableMIR.

use rustc_middle::ty::{self as rustc_ty, TyCtxt};
use rustc_smir::rustc_internal::internal;
use stable_mir::mir::{
    AggregateKind, AssertMessage, Body, BorrowKind, CastKind, ConstOperand, CopyNonOverlapping,
    CoroutineDesugaring, CoroutineKind, CoroutineSource, FakeBorrowKind, FakeReadCause, LocalDecl,
    MutBorrowKind, NonDivergingIntrinsic, NullOp, Operand, PointerCoercion, RetagKind, Rvalue,
    Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind, UnwindAction,
    UserTypeProjection, Variance,
};

pub trait RustcInternalMir {
    type T<'tcx>;
    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx>;
}

impl RustcInternalMir for AggregateKind {
    type T<'tcx> = rustc_middle::mir::AggregateKind<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            AggregateKind::Array(ty) => rustc_middle::mir::AggregateKind::Array(internal(tcx, ty)),
            AggregateKind::Tuple => rustc_middle::mir::AggregateKind::Tuple,
            AggregateKind::Adt(
                adt_def,
                variant_idx,
                generic_args,
                maybe_user_type_annotation_index,
                maybe_field_idx,
            ) => rustc_middle::mir::AggregateKind::Adt(
                internal(tcx, adt_def.0),
                internal(tcx, variant_idx),
                internal(tcx, generic_args),
                maybe_user_type_annotation_index
                    .map(rustc_middle::ty::UserTypeAnnotationIndex::from_usize),
                maybe_field_idx.map(rustc_target::abi::FieldIdx::from_usize),
            ),
            AggregateKind::Closure(closure_def, generic_args) => {
                rustc_middle::mir::AggregateKind::Closure(
                    internal(tcx, closure_def.0),
                    internal(tcx, generic_args),
                )
            }
            AggregateKind::Coroutine(coroutine_def, generic_args, _) => {
                rustc_middle::mir::AggregateKind::Coroutine(
                    internal(tcx, coroutine_def.0),
                    internal(tcx, generic_args),
                )
            }
            AggregateKind::RawPtr(ty, mutability) => rustc_middle::mir::AggregateKind::RawPtr(
                internal(tcx, ty),
                internal(tcx, mutability),
            ),
        }
    }
}

impl RustcInternalMir for ConstOperand {
    type T<'tcx> = rustc_middle::mir::ConstOperand<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        rustc_middle::mir::ConstOperand {
            span: internal(tcx, self.span),
            user_ty: self.user_ty.map(rustc_ty::UserTypeAnnotationIndex::from_usize),
            const_: internal(tcx, self.const_.clone()),
        }
    }
}

impl RustcInternalMir for Operand {
    type T<'tcx> = rustc_middle::mir::Operand<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            Operand::Copy(place) => rustc_middle::mir::Operand::Copy(internal(tcx, place)),
            Operand::Move(place) => rustc_middle::mir::Operand::Move(internal(tcx, place)),
            Operand::Constant(const_operand) => {
                rustc_middle::mir::Operand::Constant(Box::new(const_operand.internal_mir(tcx)))
            }
        }
    }
}

impl RustcInternalMir for PointerCoercion {
    type T<'tcx> = rustc_middle::ty::adjustment::PointerCoercion;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            PointerCoercion::ReifyFnPointer => {
                rustc_middle::ty::adjustment::PointerCoercion::ReifyFnPointer
            }
            PointerCoercion::UnsafeFnPointer => {
                rustc_middle::ty::adjustment::PointerCoercion::UnsafeFnPointer
            }
            PointerCoercion::ClosureFnPointer(safety) => {
                rustc_middle::ty::adjustment::PointerCoercion::ClosureFnPointer(internal(
                    tcx, safety,
                ))
            }
            PointerCoercion::MutToConstPointer => {
                rustc_middle::ty::adjustment::PointerCoercion::MutToConstPointer
            }
            PointerCoercion::ArrayToPointer => {
                rustc_middle::ty::adjustment::PointerCoercion::ArrayToPointer
            }
            PointerCoercion::Unsize => rustc_middle::ty::adjustment::PointerCoercion::Unsize,
        }
    }
}

impl RustcInternalMir for CastKind {
    type T<'tcx> = rustc_middle::mir::CastKind;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            CastKind::PointerExposeAddress => rustc_middle::mir::CastKind::PointerExposeProvenance,
            CastKind::PointerWithExposedProvenance => {
                rustc_middle::mir::CastKind::PointerWithExposedProvenance
            }
            CastKind::PointerCoercion(ptr_coercion) => {
                rustc_middle::mir::CastKind::PointerCoercion(ptr_coercion.internal_mir(tcx))
            }
            CastKind::DynStar => rustc_middle::mir::CastKind::DynStar,
            CastKind::IntToInt => rustc_middle::mir::CastKind::IntToInt,
            CastKind::FloatToInt => rustc_middle::mir::CastKind::FloatToInt,
            CastKind::FloatToFloat => rustc_middle::mir::CastKind::FloatToFloat,
            CastKind::IntToFloat => rustc_middle::mir::CastKind::IntToFloat,
            CastKind::PtrToPtr => rustc_middle::mir::CastKind::PtrToPtr,
            CastKind::FnPtrToPtr => rustc_middle::mir::CastKind::FnPtrToPtr,
            CastKind::Transmute => rustc_middle::mir::CastKind::Transmute,
        }
    }
}

impl RustcInternalMir for FakeBorrowKind {
    type T<'tcx> = rustc_middle::mir::FakeBorrowKind;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            FakeBorrowKind::Deep => rustc_middle::mir::FakeBorrowKind::Deep,
            FakeBorrowKind::Shallow => rustc_middle::mir::FakeBorrowKind::Shallow,
        }
    }
}

impl RustcInternalMir for MutBorrowKind {
    type T<'tcx> = rustc_middle::mir::MutBorrowKind;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            MutBorrowKind::Default => rustc_middle::mir::MutBorrowKind::Default,
            MutBorrowKind::TwoPhaseBorrow => rustc_middle::mir::MutBorrowKind::TwoPhaseBorrow,
            MutBorrowKind::ClosureCapture => rustc_middle::mir::MutBorrowKind::ClosureCapture,
        }
    }
}

impl RustcInternalMir for BorrowKind {
    type T<'tcx> = rustc_middle::mir::BorrowKind;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            BorrowKind::Shared => rustc_middle::mir::BorrowKind::Shared,
            BorrowKind::Fake(fake_borrow_kind) => {
                rustc_middle::mir::BorrowKind::Fake(fake_borrow_kind.internal_mir(tcx))
            }
            BorrowKind::Mut { kind } => {
                rustc_middle::mir::BorrowKind::Mut { kind: kind.internal_mir(tcx) }
            }
        }
    }
}

impl RustcInternalMir for NullOp {
    type T<'tcx> = rustc_middle::mir::NullOp<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            NullOp::SizeOf => rustc_middle::mir::NullOp::SizeOf,
            NullOp::AlignOf => rustc_middle::mir::NullOp::AlignOf,
            NullOp::OffsetOf(offsets) => rustc_middle::mir::NullOp::OffsetOf(
                tcx.mk_offset_of(
                    offsets
                        .iter()
                        .map(|(variant_idx, field_idx)| {
                            (
                                internal(tcx, variant_idx),
                                rustc_target::abi::FieldIdx::from_usize(*field_idx),
                            )
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                ),
            ),
            NullOp::UbChecks => rustc_middle::mir::NullOp::UbChecks,
        }
    }
}

impl RustcInternalMir for Rvalue {
    type T<'tcx> = rustc_middle::mir::Rvalue<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            Rvalue::AddressOf(mutability, place) => rustc_middle::mir::Rvalue::AddressOf(
                internal(tcx, mutability),
                internal(tcx, place),
            ),
            Rvalue::Aggregate(aggregate_kind, operands) => rustc_middle::mir::Rvalue::Aggregate(
                Box::new(aggregate_kind.internal_mir(tcx)),
                rustc_index::IndexVec::from_raw(
                    operands.iter().map(|operand| operand.internal_mir(tcx)).collect(),
                ),
            ),
            Rvalue::BinaryOp(bin_op, left_operand, right_operand)
            | Rvalue::CheckedBinaryOp(bin_op, left_operand, right_operand) => {
                rustc_middle::mir::Rvalue::BinaryOp(
                    internal(tcx, bin_op),
                    Box::new((left_operand.internal_mir(tcx), right_operand.internal_mir(tcx))),
                )
            }
            Rvalue::Cast(cast_kind, operand, ty) => rustc_middle::mir::Rvalue::Cast(
                cast_kind.internal_mir(tcx),
                operand.internal_mir(tcx),
                internal(tcx, ty),
            ),
            Rvalue::CopyForDeref(place) => {
                rustc_middle::mir::Rvalue::CopyForDeref(internal(tcx, place))
            }
            Rvalue::Discriminant(place) => {
                rustc_middle::mir::Rvalue::Discriminant(internal(tcx, place))
            }
            Rvalue::Len(place) => rustc_middle::mir::Rvalue::Len(internal(tcx, place)),
            Rvalue::Ref(region, borrow_kind, place) => rustc_middle::mir::Rvalue::Ref(
                internal(tcx, region),
                borrow_kind.internal_mir(tcx),
                internal(tcx, place),
            ),
            Rvalue::Repeat(operand, ty_const) => rustc_middle::mir::Rvalue::Repeat(
                operand.internal_mir(tcx),
                internal(tcx, ty_const),
            ),
            Rvalue::ShallowInitBox(operand, ty) => rustc_middle::mir::Rvalue::ShallowInitBox(
                operand.internal_mir(tcx),
                internal(tcx, ty),
            ),
            Rvalue::ThreadLocalRef(crate_item) => {
                rustc_middle::mir::Rvalue::ThreadLocalRef(internal(tcx, crate_item.0))
            }
            Rvalue::NullaryOp(null_op, ty) => {
                rustc_middle::mir::Rvalue::NullaryOp(null_op.internal_mir(tcx), internal(tcx, ty))
            }
            Rvalue::UnaryOp(un_op, operand) => {
                rustc_middle::mir::Rvalue::UnaryOp(internal(tcx, un_op), operand.internal_mir(tcx))
            }
            Rvalue::Use(operand) => rustc_middle::mir::Rvalue::Use(operand.internal_mir(tcx)),
        }
    }
}

impl RustcInternalMir for FakeReadCause {
    type T<'tcx> = rustc_middle::mir::FakeReadCause;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            FakeReadCause::ForMatchGuard => rustc_middle::mir::FakeReadCause::ForMatchGuard,
            FakeReadCause::ForMatchedPlace(_opaque) => {
                unimplemented!("cannot convert back from an opaque field")
            }
            FakeReadCause::ForGuardBinding => rustc_middle::mir::FakeReadCause::ForGuardBinding,
            FakeReadCause::ForLet(_opaque) => {
                unimplemented!("cannot convert back from an opaque field")
            }
            FakeReadCause::ForIndex => rustc_middle::mir::FakeReadCause::ForIndex,
        }
    }
}

impl RustcInternalMir for RetagKind {
    type T<'tcx> = rustc_middle::mir::RetagKind;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            RetagKind::FnEntry => rustc_middle::mir::RetagKind::FnEntry,
            RetagKind::TwoPhase => rustc_middle::mir::RetagKind::TwoPhase,
            RetagKind::Raw => rustc_middle::mir::RetagKind::Raw,
            RetagKind::Default => rustc_middle::mir::RetagKind::Default,
        }
    }
}

impl RustcInternalMir for UserTypeProjection {
    type T<'tcx> = rustc_middle::mir::UserTypeProjection;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        unimplemented!("cannot convert back from an opaque field")
    }
}

impl RustcInternalMir for Variance {
    type T<'tcx> = rustc_middle::ty::Variance;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            Variance::Covariant => rustc_middle::ty::Variance::Covariant,
            Variance::Invariant => rustc_middle::ty::Variance::Invariant,
            Variance::Contravariant => rustc_middle::ty::Variance::Contravariant,
            Variance::Bivariant => rustc_middle::ty::Variance::Bivariant,
        }
    }
}

impl RustcInternalMir for CopyNonOverlapping {
    type T<'tcx> = rustc_middle::mir::CopyNonOverlapping<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        rustc_middle::mir::CopyNonOverlapping {
            src: self.src.internal_mir(tcx),
            dst: self.dst.internal_mir(tcx),
            count: self.count.internal_mir(tcx),
        }
    }
}

impl RustcInternalMir for NonDivergingIntrinsic {
    type T<'tcx> = rustc_middle::mir::NonDivergingIntrinsic<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            NonDivergingIntrinsic::Assume(operand) => {
                rustc_middle::mir::NonDivergingIntrinsic::Assume(operand.internal_mir(tcx))
            }
            NonDivergingIntrinsic::CopyNonOverlapping(copy_non_overlapping) => {
                rustc_middle::mir::NonDivergingIntrinsic::CopyNonOverlapping(
                    copy_non_overlapping.internal_mir(tcx),
                )
            }
        }
    }
}

impl RustcInternalMir for StatementKind {
    type T<'tcx> = rustc_middle::mir::StatementKind<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            StatementKind::Assign(place, rvalue) => rustc_middle::mir::StatementKind::Assign(
                Box::new((internal(tcx, place), rvalue.internal_mir(tcx))),
            ),
            StatementKind::FakeRead(fake_read_cause, place) => {
                rustc_middle::mir::StatementKind::FakeRead(Box::new((
                    fake_read_cause.internal_mir(tcx),
                    internal(tcx, place),
                )))
            }
            StatementKind::SetDiscriminant { place, variant_index } => {
                rustc_middle::mir::StatementKind::SetDiscriminant {
                    place: internal(tcx, place).into(),
                    variant_index: internal(tcx, variant_index),
                }
            }
            StatementKind::Deinit(place) => {
                rustc_middle::mir::StatementKind::Deinit(internal(tcx, place).into())
            }
            StatementKind::StorageLive(local) => rustc_middle::mir::StatementKind::StorageLive(
                rustc_middle::mir::Local::from_usize(*local),
            ),
            StatementKind::StorageDead(local) => rustc_middle::mir::StatementKind::StorageDead(
                rustc_middle::mir::Local::from_usize(*local),
            ),
            StatementKind::Retag(retag_kind, place) => rustc_middle::mir::StatementKind::Retag(
                retag_kind.internal_mir(tcx),
                internal(tcx, place).into(),
            ),
            StatementKind::PlaceMention(place) => {
                rustc_middle::mir::StatementKind::PlaceMention(Box::new(internal(tcx, place)))
            }
            StatementKind::AscribeUserType { place, projections, variance } => {
                rustc_middle::mir::StatementKind::AscribeUserType(
                    Box::new((internal(tcx, place), projections.internal_mir(tcx))),
                    variance.internal_mir(tcx),
                )
            }
            StatementKind::Coverage(_coverage_kind) => {
                unimplemented!("cannot convert back from an opaque field")
            }
            StatementKind::Intrinsic(non_diverging_intrinsic) => {
                rustc_middle::mir::StatementKind::Intrinsic(
                    non_diverging_intrinsic.internal_mir(tcx).into(),
                )
            }
            StatementKind::ConstEvalCounter => rustc_middle::mir::StatementKind::ConstEvalCounter,
            StatementKind::Nop => rustc_middle::mir::StatementKind::Nop,
        }
    }
}

impl RustcInternalMir for Statement {
    type T<'tcx> = rustc_middle::mir::Statement<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        rustc_middle::mir::Statement {
            source_info: rustc_middle::mir::SourceInfo::outermost(internal(tcx, self.span)),
            kind: self.kind.internal_mir(tcx),
        }
    }
}

impl RustcInternalMir for UnwindAction {
    type T<'tcx> = rustc_middle::mir::UnwindAction;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            UnwindAction::Continue => rustc_middle::mir::UnwindAction::Continue,
            UnwindAction::Unreachable => rustc_middle::mir::UnwindAction::Unreachable,
            UnwindAction::Terminate => rustc_middle::mir::UnwindAction::Terminate(
                rustc_middle::mir::UnwindTerminateReason::Abi,
            ),
            UnwindAction::Cleanup(basic_block_idx) => rustc_middle::mir::UnwindAction::Cleanup(
                rustc_middle::mir::BasicBlock::from_usize(*basic_block_idx),
            ),
        }
    }
}

impl RustcInternalMir for SwitchTargets {
    type T<'tcx> = rustc_middle::mir::SwitchTargets;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        rustc_middle::mir::SwitchTargets::new(
            self.branches().map(|(value, basic_block_idx)| {
                (value, rustc_middle::mir::BasicBlock::from_usize(basic_block_idx))
            }),
            rustc_middle::mir::BasicBlock::from_usize(self.otherwise()),
        )
    }
}

impl RustcInternalMir for CoroutineDesugaring {
    type T<'tcx> = rustc_hir::CoroutineDesugaring;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            CoroutineDesugaring::Async => rustc_hir::CoroutineDesugaring::Async,
            CoroutineDesugaring::Gen => rustc_hir::CoroutineDesugaring::Gen,
            CoroutineDesugaring::AsyncGen => rustc_hir::CoroutineDesugaring::AsyncGen,
        }
    }
}

impl RustcInternalMir for CoroutineSource {
    type T<'tcx> = rustc_hir::CoroutineSource;

    fn internal_mir<'tcx>(&self, _tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            CoroutineSource::Block => rustc_hir::CoroutineSource::Block,
            CoroutineSource::Closure => rustc_hir::CoroutineSource::Closure,
            CoroutineSource::Fn => rustc_hir::CoroutineSource::Fn,
        }
    }
}

impl RustcInternalMir for CoroutineKind {
    type T<'tcx> = rustc_hir::CoroutineKind;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            CoroutineKind::Desugared(coroutine_desugaring, coroutine_source) => {
                rustc_hir::CoroutineKind::Desugared(
                    coroutine_desugaring.internal_mir(tcx),
                    coroutine_source.internal_mir(tcx),
                )
            }
            CoroutineKind::Coroutine(movability) => {
                rustc_hir::CoroutineKind::Coroutine(internal(tcx, movability))
            }
        }
    }
}

impl RustcInternalMir for AssertMessage {
    type T<'tcx> = rustc_middle::mir::AssertMessage<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            AssertMessage::BoundsCheck { len, index } => {
                rustc_middle::mir::AssertMessage::BoundsCheck {
                    len: len.internal_mir(tcx),
                    index: index.internal_mir(tcx),
                }
            }
            AssertMessage::Overflow(bin_op, left_operand, right_operand) => {
                rustc_middle::mir::AssertMessage::Overflow(
                    internal(tcx, bin_op),
                    left_operand.internal_mir(tcx),
                    right_operand.internal_mir(tcx),
                )
            }
            AssertMessage::OverflowNeg(operand) => {
                rustc_middle::mir::AssertMessage::OverflowNeg(operand.internal_mir(tcx))
            }
            AssertMessage::DivisionByZero(operand) => {
                rustc_middle::mir::AssertMessage::DivisionByZero(operand.internal_mir(tcx))
            }
            AssertMessage::RemainderByZero(operand) => {
                rustc_middle::mir::AssertMessage::RemainderByZero(operand.internal_mir(tcx))
            }
            AssertMessage::ResumedAfterReturn(coroutine_kind) => {
                rustc_middle::mir::AssertMessage::ResumedAfterReturn(
                    coroutine_kind.internal_mir(tcx),
                )
            }
            AssertMessage::ResumedAfterPanic(coroutine_kind) => {
                rustc_middle::mir::AssertMessage::ResumedAfterPanic(
                    coroutine_kind.internal_mir(tcx),
                )
            }
            AssertMessage::MisalignedPointerDereference { required, found } => {
                rustc_middle::mir::AssertMessage::MisalignedPointerDereference {
                    required: required.internal_mir(tcx),
                    found: found.internal_mir(tcx),
                }
            }
        }
    }
}

impl RustcInternalMir for TerminatorKind {
    type T<'tcx> = rustc_middle::mir::TerminatorKind<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        match self {
            TerminatorKind::Goto { target } => rustc_middle::mir::TerminatorKind::Goto {
                target: rustc_middle::mir::BasicBlock::from_usize(*target),
            },
            TerminatorKind::SwitchInt { discr, targets } => {
                rustc_middle::mir::TerminatorKind::SwitchInt {
                    discr: discr.internal_mir(tcx),
                    targets: targets.internal_mir(tcx),
                }
            }
            TerminatorKind::Resume => rustc_middle::mir::TerminatorKind::UnwindResume,
            TerminatorKind::Abort => rustc_middle::mir::TerminatorKind::UnwindTerminate(
                rustc_middle::mir::UnwindTerminateReason::Abi,
            ),
            TerminatorKind::Return => rustc_middle::mir::TerminatorKind::Return,
            TerminatorKind::Unreachable => rustc_middle::mir::TerminatorKind::Unreachable,
            TerminatorKind::Drop { place, target, unwind } => {
                rustc_middle::mir::TerminatorKind::Drop {
                    place: internal(tcx, place),
                    target: rustc_middle::mir::BasicBlock::from_usize(*target),
                    unwind: unwind.internal_mir(tcx),
                    replace: false,
                }
            }
            TerminatorKind::Call { func, args, destination, target, unwind } => {
                rustc_middle::mir::TerminatorKind::Call {
                    func: func.internal_mir(tcx),
                    args: Box::from_iter(
                        args.iter().map(|arg| {
                            rustc_span::source_map::dummy_spanned(arg.internal_mir(tcx))
                        }),
                    ),
                    destination: internal(tcx, destination),
                    target: target.map(|basic_block_idx| {
                        rustc_middle::mir::BasicBlock::from_usize(basic_block_idx)
                    }),
                    unwind: unwind.internal_mir(tcx),
                    call_source: rustc_middle::mir::CallSource::Normal,
                    fn_span: rustc_span::DUMMY_SP,
                }
            }
            TerminatorKind::Assert { cond, expected, msg, target, unwind } => {
                rustc_middle::mir::TerminatorKind::Assert {
                    cond: cond.internal_mir(tcx),
                    expected: *expected,
                    msg: Box::new(msg.internal_mir(tcx)),
                    target: rustc_middle::mir::BasicBlock::from_usize(*target),
                    unwind: unwind.internal_mir(tcx),
                }
            }
            TerminatorKind::InlineAsm { .. } => todo!(),
        }
    }
}

impl RustcInternalMir for Terminator {
    type T<'tcx> = rustc_middle::mir::Terminator<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        rustc_middle::mir::Terminator {
            source_info: rustc_middle::mir::SourceInfo::outermost(internal(tcx, self.span)),
            kind: self.kind.internal_mir(tcx),
        }
    }
}

impl RustcInternalMir for LocalDecl {
    type T<'tcx> = rustc_middle::mir::LocalDecl<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        rustc_middle::mir::LocalDecl {
            mutability: internal(tcx, self.mutability),
            local_info: rustc_middle::mir::ClearCrossCrate::Set(Box::new(
                rustc_middle::mir::LocalInfo::Boring,
            )),
            ty: internal(tcx, self.ty),
            user_ty: None,
            source_info: rustc_middle::mir::SourceInfo::outermost(internal(tcx, self.span)),
        }
    }
}

impl RustcInternalMir for Body {
    type T<'tcx> = rustc_middle::mir::Body<'tcx>;

    fn internal_mir<'tcx>(&self, tcx: TyCtxt<'tcx>) -> Self::T<'tcx> {
        let internal_basic_blocks = rustc_index::IndexVec::from_raw(
            self.blocks
                .iter()
                .map(|stable_basic_block| rustc_middle::mir::BasicBlockData {
                    statements: stable_basic_block
                        .statements
                        .iter()
                        .map(|statement| statement.internal_mir(tcx))
                        .collect(),
                    terminator: Some(stable_basic_block.terminator.internal_mir(tcx)),
                    is_cleanup: false,
                })
                .collect(),
        );
        let local_decls = rustc_index::IndexVec::from_raw(
            self.locals().iter().map(|local_decl| local_decl.internal_mir(tcx)).collect(),
        );
        rustc_middle::mir::Body::new(
            rustc_middle::mir::MirSource::item(rustc_hir::def_id::CRATE_DEF_ID.to_def_id()),
            internal_basic_blocks,
            rustc_index::IndexVec::new(),
            local_decls,
            rustc_index::IndexVec::new(),
            self.arg_locals().len(),
            Vec::new(),
            rustc_span::DUMMY_SP,
            None,
            None,
        )
    }
}
