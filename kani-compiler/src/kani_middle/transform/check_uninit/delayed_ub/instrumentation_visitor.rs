// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Visitor that collects all instructions relevant to uninitialized memory access caused by delayed
//! UB. In practice, that means collecting all instructions where the place is featured.

use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_middle::transform::check_uninit::relevant_instruction::{
    InitRelevantInstruction, MemoryInitOp,
};
use crate::kani_middle::transform::check_uninit::TargetFinder;

use stable_mir::mir::visit::{Location, PlaceContext};
use stable_mir::mir::{BasicBlockIdx, MirVisitor, Operand, Place, ProjectionElem, Statement};

pub struct DelayedUbTargetVisitor<'a> {
    /// Whether we should skip the next instruction, since it might've been instrumented already.
    /// When we instrument an instruction, we partition the basic block, and the instruction that
    /// may trigger UB becomes the first instruction of the basic block, which we need to skip
    /// later.
    skip_next: bool,
    /// The instruction being visited at a given point.
    current: SourceInstruction,
    /// The target instruction that should be verified.
    pub target: Option<InitRelevantInstruction>,
    /// The list of places we should be looking for, ignoring others.
    place_filter: &'a [Place],
}

impl<'a> TargetFinder for DelayedUbTargetVisitor<'a> {
    fn find_next(
        body: &MutableBody,
        bb: BasicBlockIdx,
        skip_first: bool,
        place_filter: &[Place],
    ) -> Option<InitRelevantInstruction> {
        let mut visitor = DelayedUbTargetVisitor {
            skip_next: skip_first,
            current: SourceInstruction::Statement { idx: 0, bb },
            target: None,
            place_filter,
        };
        visitor.visit_basic_block(&body.blocks()[bb]);
        visitor.target
    }
}

impl<'a> DelayedUbTargetVisitor<'a> {
    fn push_target(&mut self, source_op: MemoryInitOp) {
        let target = self.target.get_or_insert_with(|| InitRelevantInstruction {
            source: self.current,
            after_instruction: vec![],
            before_instruction: vec![],
        });
        target.push_operation(source_op);
    }
}

impl<'a> MirVisitor for DelayedUbTargetVisitor<'a> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        if self.skip_next {
            self.skip_next = false;
        } else if self.target.is_none() {
            // Check all inner places.
            self.super_statement(stmt, location);
            // Switch to the next statement.
            let SourceInstruction::Statement { idx, bb } = self.current else { unreachable!() };
            self.current = SourceInstruction::Statement { idx: idx + 1, bb };
        }
    }

    fn visit_place(&mut self, place: &Place, ptx: PlaceContext, location: Location) {
        // Match the place by its local.
        if self
            .place_filter
            .iter()
            .any(|instrumented_place| instrumented_place.local == place.local)
        {
            let deref_projection_detected = place
                .projection
                .iter()
                .any(|projection_elem| matches!(projection_elem, ProjectionElem::Deref));
            // We should only track the place itself, not whatever it gets dereferenced to.
            if !deref_projection_detected {
                // If we are mutating the place, initialize it.
                if ptx.is_mutating() {
                    self.push_target(MemoryInitOp::SetRef {
                        operand: Operand::Copy(place.clone()),
                        value: true,
                        position: InsertPosition::After,
                    });
                } else {
                    // Otherwise, check its initialization.
                    self.push_target(MemoryInitOp::CheckRef {
                        operand: Operand::Copy(place.clone()),
                    });
                }
            }
        }
        self.super_place(place, ptx, location)
    }
}
