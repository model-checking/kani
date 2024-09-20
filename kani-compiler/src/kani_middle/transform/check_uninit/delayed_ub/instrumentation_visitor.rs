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
    fn find_all(mut self, body: &MutableBody) -> Vec<InitRelevantInstruction> {
        for (bb_idx, bb) in body.blocks().iter().enumerate() {
            self.current_target = InitRelevantInstruction {
                source: SourceInstruction::Statement { idx: 0, bb: bb_idx },
                before_instruction: vec![],
                after_instruction: vec![],
            };
            self.visit_basic_block(bb);
        }
        self.targets
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
        self.current_target.push_operation(source_op);
    }
}

impl<'a, 'tcx> MirVisitor for InstrumentationVisitor<'a, 'tcx> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        self.super_statement(stmt, location);
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
            unreachable!()
        }
        self.super_terminator(term, location);
        // Push the current target from the terminator onto the list.
        self.targets.push(self.current_target.clone());
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
        // In order to check whether we should get-instrument the place, see if it resolves to the
        // analysis target.
        let needs_get = {
            self.points_to
                .resolve_place_stable(place.clone(), self.current_instance, self.tcx)
                .intersection(&self.analysis_targets)
                .next()
                .is_some()
        };

        // In order to check whether we should set-instrument the place, we need to figure out if
        // the place has a common ancestor of the same level with the target.
        //
        // This is needed because instrumenting the place only if it resolves to the target could give
        // false positives in presence of some aliasing relations.
        //
        // Here is a simple example:
        // ```
        // fn foo(val_1: u32, val_2: u32, flag: bool) {
        //   let reference = if flag {
        //     &val_1
        //   } else {
        //     &val_2
        //   };
        //   let _ = *reference;
        // }
        // ```
        // It yields the following aliasing graph:
        //
        // `val_1 <-- reference --> val_2`
        //
        // If `val_1` is a legitimate instrumentation target, we would get-instrument an instruction
        // that reads from `*reference`, but that could mean that `val_2` is checked, too. Hence,
        // if we don't set-instrument `val_2` we will get a false-positive.
        //
        // See `tests/expected/uninit/delayed-ub-overapprox.rs` for a more specific example.
        let needs_set = {
            let mut has_common_ancestor = false;
            let mut self_ancestors =
                self.points_to.resolve_place_stable(place.clone(), self.current_instance, self.tcx);
            let mut target_ancestors = self.analysis_targets.clone();

            while !self_ancestors.is_empty() || !target_ancestors.is_empty() {
                if self_ancestors.intersection(&target_ancestors).next().is_some() {
                    has_common_ancestor = true;
                    break;
                }
                self_ancestors = self.points_to.ancestors(&self_ancestors);
                target_ancestors = self.points_to.ancestors(&target_ancestors);
            }

            has_common_ancestor
        };

        // If we are mutating the place, initialize it.
        if ptx.is_mutating() && needs_set {
            self.push_target(MemoryInitOp::SetRef {
                operand: Operand::Copy(place.clone()),
                value: true,
                position: InsertPosition::After,
            });
        } else if !ptx.is_mutating() && needs_get {
            // Otherwise, check its initialization.
            self.push_target(MemoryInitOp::CheckRef { operand: Operand::Copy(place.clone()) });
        }
        self.super_place(place, ptx, location)
    }
}
