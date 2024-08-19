// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Visitor that collects all instructions relevant to uninitialized memory access caused by delayed
//! UB. In practice, that means collecting all instructions where the place is featured.

use crate::kani_middle::{
    points_to::{MemLoc, PointsToGraph},
    transform::{
        body::{InsertPosition, MutableBody, SourceInstruction},
        check_uninit::{
            relevant_instruction::{InitRelevantInstruction, MemoryInitOp},
            TargetFinder,
        },
    },
};
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::{
    mono::Instance, BasicBlock, CopyNonOverlapping, InlineAsmOperand, NonDivergingIntrinsic,
    Operand, Place, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
};
use std::collections::HashSet;

pub struct InstrumentationVisitor<'a, 'tcx> {
    /// The target instruction that should be verified.
    pub target: Option<InitRelevantInstruction>,
    /// Aliasing analysis data.
    points_to: &'a PointsToGraph<'tcx>,
    /// The list of places we should be looking for, ignoring others
    analysis_targets: &'a HashSet<MemLoc<'tcx>>,
    current_instance: Instance,
    tcx: TyCtxt<'tcx>,
}

enum PlaceOperation {
    Read,
    Write,
    Deinit,
    Noop,
}

impl<'a, 'tcx> TargetFinder for InstrumentationVisitor<'a, 'tcx> {
    fn find_next(
        &mut self,
        body: &MutableBody,
        source: &SourceInstruction,
    ) -> Option<InitRelevantInstruction> {
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

impl<'a, 'tcx> InstrumentationVisitor<'a, 'tcx> {
    pub fn new(
        points_to: &'a PointsToGraph<'tcx>,
        analysis_targets: &'a HashSet<MemLoc<'tcx>>,
        current_instance: Instance,
        tcx: TyCtxt<'tcx>,
    ) -> Self {
        Self { target: None, points_to, analysis_targets, current_instance, tcx }
    }
    fn push_target(&mut self, source_op: MemoryInitOp) {
        let target = self.target.get_or_insert_with(|| InitRelevantInstruction {
            after_instruction: vec![],
            before_instruction: vec![],
        });
        target.push_operation(source_op);
    }
}

impl<'a, 'tcx> InstrumentationVisitor<'a, 'tcx> {
    fn check_statement(&mut self, stmt: &Statement) {
        let Statement { kind, .. } = stmt;
        match kind {
            StatementKind::Assign(place, rvalue) => {
                self.check_place(place, PlaceOperation::Write);
                self.check_rvalue(rvalue);
            }
            StatementKind::FakeRead(_, place) => {
                // According to the compiler docs, "When executed at runtime this is a nop." For
                // more info, see
                // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.StatementKind.html#variant.FakeRead,
                self.check_place(place, PlaceOperation::Noop);
            }
            StatementKind::SetDiscriminant { place, .. } => {
                self.check_place(place, PlaceOperation::Write);
            }
            StatementKind::Deinit(place) => {
                self.check_place(place, PlaceOperation::Deinit);
            }
            StatementKind::Retag(_, place) => {
                // According to the compiler docs, "These statements are currently only interpreted
                // by miri and only generated when -Z mir-emit-retag is passed." For more info, see
                // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.StatementKind.html#variant.Retag,
                self.check_place(place, PlaceOperation::Noop);
            }
            StatementKind::PlaceMention(place) => {
                // According to the compiler docs, "When executed at runtime, this computes the
                // given place, but then discards it without doing a load." For more info, see
                // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.StatementKind.html#variant.PlaceMention,
                self.check_place(place, PlaceOperation::Noop);
            }
            StatementKind::AscribeUserType { place, .. } => {
                // According to the compiler docs, "When executed at runtime this is a nop." For
                // more info, see
                // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.StatementKind.html#variant.AscribeUserType,
                self.check_place(place, PlaceOperation::Noop);
            }

            StatementKind::Intrinsic(intrisic) => match intrisic {
                NonDivergingIntrinsic::Assume(operand) => {
                    self.check_operand(operand);
                }
                NonDivergingIntrinsic::CopyNonOverlapping(CopyNonOverlapping {
                    src,
                    dst,
                    count,
                }) => {
                    self.check_operand(src);
                    self.check_operand(dst);
                    self.check_operand(count);
                }
            },
            StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop => {}
        }
    }

    fn check_terminator(&mut self, term: &Terminator) {
        let Terminator { kind, .. } = term;
        match kind {
            TerminatorKind::Goto { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Abort
            | TerminatorKind::Unreachable
            | TerminatorKind::Return => {}
            TerminatorKind::Assert { cond, .. } => {
                self.check_operand(cond);
            }
            TerminatorKind::Drop { place, .. } => {
                // According to the documentation, "After drop elaboration: Drop terminators are a
                // complete nop for types that have no drop glue. For other types, Drop terminators
                // behave exactly like a call to core::mem::drop_in_place with a pointer to the
                // given place." Since we check arguments when instrumenting function calls, perhaps
                // we need to check that one, too. For more info, see:
                // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.TerminatorKind.html#variant.Drop
                self.check_place(place, PlaceOperation::Read);
            }
            TerminatorKind::Call { func, args, destination, target: _, unwind: _ } => {
                self.check_operand(func);
                for arg in args {
                    self.check_operand(arg);
                }
                self.check_place(destination, PlaceOperation::Write);
            }
            TerminatorKind::InlineAsm { operands, .. } => {
                for op in operands {
                    let InlineAsmOperand { in_value, out_place, raw_rpr: _ } = op;
                    if let Some(input) = in_value {
                        self.check_operand(input);
                    }
                    if let Some(output) = out_place {
                        self.check_place(output, PlaceOperation::Write);
                    }
                }
            }
            TerminatorKind::SwitchInt { discr, .. } => {
                self.check_operand(discr);
            }
        }
    }

    fn check_rvalue(&mut self, rvalue: &Rvalue) {
        match rvalue {
            Rvalue::AddressOf(_, place) | Rvalue::Ref(_, _, place) => {
                self.check_place(place, PlaceOperation::Noop);
            }
            Rvalue::Aggregate(_, operands) => {
                for op in operands {
                    self.check_operand(op);
                }
            }
            Rvalue::BinaryOp(_, lhs, rhs) | Rvalue::CheckedBinaryOp(_, lhs, rhs) => {
                self.check_operand(lhs);
                self.check_operand(rhs);
            }
            Rvalue::Cast(_, op, _)
            | Rvalue::Repeat(op, _)
            | Rvalue::ShallowInitBox(op, ..)
            | Rvalue::UnaryOp(_, op)
            | Rvalue::Use(op) => {
                self.check_operand(op);
            }
            Rvalue::CopyForDeref(place) | Rvalue::Discriminant(place) | Rvalue::Len(place) => {
                self.check_place(place, PlaceOperation::Read);
            }
            Rvalue::ThreadLocalRef(..) | Rvalue::NullaryOp(..) => {}
        }
    }

    fn check_operand(&mut self, operand: &Operand) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                self.check_place(place, PlaceOperation::Read)
            }
            Operand::Constant(_) => {
                // Those should be safe to skip, as they are either constants or statics. In the
                // latter case, we handle them in regular uninit visior
            }
        }
    }

    fn check_place(&mut self, place: &Place, place_operation: PlaceOperation) {
        // Match the place by whatever it is pointing to and find an intersection with the targets.
        if self
            .points_to
            .resolve_place_stable(place.clone(), self.current_instance, self.tcx)
            .intersection(&self.analysis_targets)
            .next()
            .is_some()
        {
            match place_operation {
                PlaceOperation::Write => {
                    // If we are mutating the place, initialize it.
                    self.push_target(MemoryInitOp::SetRef {
                        operand: Operand::Copy(place.clone()),
                        value: true,
                        position: InsertPosition::After,
                    })
                }
                PlaceOperation::Deinit => {
                    // If we are mutating the place, initialize it.
                    self.push_target(MemoryInitOp::SetRef {
                        operand: Operand::Copy(place.clone()),
                        value: false,
                        position: InsertPosition::After,
                    })
                }
                PlaceOperation::Read => {
                    // Otherwise, check its initialization.
                    self.push_target(MemoryInitOp::CheckRef {
                        operand: Operand::Copy(place.clone()),
                    });
                }
                PlaceOperation::Noop => {}
            }
        }
    }
}
