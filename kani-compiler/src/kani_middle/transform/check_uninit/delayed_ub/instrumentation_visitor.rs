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
    BasicBlock, MirVisitor, Operand, Place, Rvalue,
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
                self.visit_statement(stmt, self.__location_hack_remove_before_merging(stmt.span))
            }
            SourceInstruction::Terminator { bb } => {
                let BasicBlock { terminator, .. } = &body.blocks()[bb];
                self.visit_terminator(
                    terminator,
                    self.__location_hack_remove_before_merging(terminator.span),
                )
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

impl<'a, 'tcx> MirVisitor for InstrumentationVisitor<'a, 'tcx> {
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
