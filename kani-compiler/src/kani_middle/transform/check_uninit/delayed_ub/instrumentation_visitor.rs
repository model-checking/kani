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
    mono::Instance,
    visit::{Location, PlaceContext},
    MirVisitor, Operand, Place, Rvalue, Statement, Terminator,
};
use std::collections::HashSet;

pub struct InstrumentationVisitor<'a, 'tcx> {
    /// All target instructions in the body.
    targets: Vec<InitRelevantInstruction>,
    /// Currently analyzed instruction.
    current_instruction: SourceInstruction,
    /// Current analysis target, eventually needs to be added to a list of all targets.
    current_target: InitRelevantInstruction,
    /// Aliasing analysis data.
    points_to: &'a PointsToGraph<'tcx>,
    /// The list of places we should be looking for, ignoring others
    analysis_targets: &'a HashSet<MemLoc<'tcx>>,
    current_instance: Instance,
    tcx: TyCtxt<'tcx>,
}

impl<'a, 'tcx> TargetFinder for InstrumentationVisitor<'a, 'tcx> {
    fn find_all(&mut self, body: &MutableBody) -> Vec<InitRelevantInstruction> {
        for (bb_idx, bb) in body.blocks().iter().enumerate() {
            self.current_instruction = SourceInstruction::Statement { idx: 0, bb: bb_idx };
            self.visit_basic_block(bb);
        }
        // Push the last current target into the list.
        self.targets.push(self.current_target.clone());
        self.targets.clone()
    }
}

impl<'a, 'tcx> InstrumentationVisitor<'a, 'tcx> {
    pub fn new(
        points_to: &'a PointsToGraph<'tcx>,
        analysis_targets: &'a HashSet<MemLoc<'tcx>>,
        current_instance: Instance,
        tcx: TyCtxt<'tcx>,
    ) -> Self {
        Self {
            targets: vec![],
            current_instruction: SourceInstruction::Statement { idx: 0, bb: 0 },
            current_target: InitRelevantInstruction {
                source: SourceInstruction::Statement { idx: 0, bb: 0 },
                before_instruction: vec![],
                after_instruction: vec![],
            },
            points_to,
            analysis_targets,
            current_instance,
            tcx,
        }
    }

    fn push_target(&mut self, source_op: MemoryInitOp) {
        // If we switched to the next instruction, push the old one onto the list of targets.
        if self.current_target.source != self.current_instruction {
            self.targets.push(self.current_target.clone());
            self.current_target = InitRelevantInstruction {
                source: self.current_instruction,
                after_instruction: vec![],
                before_instruction: vec![],
            };
        }
        self.current_target.push_operation(source_op);
    }
}

impl<'a, 'tcx> MirVisitor for InstrumentationVisitor<'a, 'tcx> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        self.super_statement(stmt, location);
        // Switch to the next statement.
        if let SourceInstruction::Statement { idx, bb } = self.current_instruction {
            self.current_instruction = SourceInstruction::Statement { idx: idx + 1, bb }
        } else {
            unreachable!()
        }
    }

    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if let SourceInstruction::Statement { bb, .. } = self.current_instruction {
            self.current_instruction = SourceInstruction::Terminator { bb };
        } else {
            unreachable!()
        }
        self.super_terminator(term, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        match rvalue {
            Rvalue::AddressOf(..) | Rvalue::Ref(..) => {
                // These operations are always legitimate for us.
            }
            _ => self.super_rvalue(rvalue, location),
        }
    }

    fn visit_place(&mut self, place: &Place, ptx: PlaceContext, location: Location) {
        // Match the place by whatever it is pointing to and find an intersection with the targets.
        if self
            .points_to
            .resolve_place_stable(place.clone(), self.current_instance, self.tcx)
            .intersection(&self.analysis_targets)
            .next()
            .is_some()
        {
            // If we are mutating the place, initialize it.
            if ptx.is_mutating() {
                self.push_target(MemoryInitOp::SetRef {
                    operand: Operand::Copy(place.clone()),
                    value: true,
                    position: InsertPosition::After,
                });
            } else {
                // Otherwise, check its initialization.
                self.push_target(MemoryInitOp::CheckRef { operand: Operand::Copy(place.clone()) });
            }
        }
        self.super_place(place, ptx, location)
    }
}
