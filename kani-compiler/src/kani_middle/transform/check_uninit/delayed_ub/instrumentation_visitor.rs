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
use rustc_hir::def_id::DefId as InternalDefId;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::{
    visit::{Location, PlaceContext},
    BasicBlockIdx, MirVisitor, Operand, Place, Rvalue, Statement, Terminator,
};
use std::collections::HashSet;

pub struct InstrumentationVisitor<'a, 'tcx> {
    /// Whether we should skip the next instruction, since it might've been instrumented already.
    /// When we instrument an instruction, we partition the basic block, and the instruction that
    /// may trigger UB becomes the first instruction of the basic block, which we need to skip
    /// later.
    skip_next: bool,
    /// The instruction being visited at a given point.
    current: SourceInstruction,
    /// The target instruction that should be verified.
    pub target: Option<InitRelevantInstruction>,
    /// Aliasing analysis data.
    points_to: &'a PointsToGraph<'tcx>,
    /// The list of places we should be looking for, ignoring others
    analysis_targets: &'a HashSet<MemLoc<'tcx>>,
    current_def_id: InternalDefId,
    tcx: TyCtxt<'tcx>,
}

impl<'a, 'tcx> TargetFinder for InstrumentationVisitor<'a, 'tcx> {
    fn find_next(
        &mut self,
        body: &MutableBody,
        bb: BasicBlockIdx,
        skip_first: bool,
    ) -> Option<InitRelevantInstruction> {
        self.skip_next = skip_first;
        self.current = SourceInstruction::Statement { idx: 0, bb };
        self.target = None;
        self.visit_basic_block(&body.blocks()[bb]);
        self.target.clone()
    }
}

impl<'a, 'tcx> InstrumentationVisitor<'a, 'tcx> {
    pub fn new(
        points_to: &'a PointsToGraph<'tcx>,
        analysis_targets: &'a HashSet<MemLoc<'tcx>>,
        current_def_id: InternalDefId,
        tcx: TyCtxt<'tcx>,
    ) -> Self {
        Self {
            skip_next: false,
            current: SourceInstruction::Statement { idx: 0, bb: 0 },
            target: None,
            points_to,
            analysis_targets,
            current_def_id,
            tcx,
        }
    }
    fn push_target(&mut self, source_op: MemoryInitOp) {
        let target = self.target.get_or_insert_with(|| InitRelevantInstruction {
            source: self.current,
            after_instruction: vec![],
            before_instruction: vec![],
        });
        target.push_operation(source_op);
    }
}

impl<'a, 'tcx> MirVisitor for InstrumentationVisitor<'a, 'tcx> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        if self.skip_next {
            self.skip_next = false;
        } else if self.target.is_none() {
            // Check all inner places.
            self.super_statement(stmt, location);
        }
        // Switch to the next statement.
        let SourceInstruction::Statement { idx, bb } = self.current else { unreachable!() };
        self.current = SourceInstruction::Statement { idx: idx + 1, bb };
    }

    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if !(self.skip_next || self.target.is_some()) {
            self.current = SourceInstruction::Terminator { bb: self.current.bb() };
            self.super_terminator(term, location);
        }
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
            .follow_from_place(rustc_internal::internal(self.tcx, place), self.current_def_id)
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
