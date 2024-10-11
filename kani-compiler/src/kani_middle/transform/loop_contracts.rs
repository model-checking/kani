// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains code related to the MIR-to-MIR pass to enable loop contracts.
//!

use crate::kani_middle::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::TransformationType;
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_queries::QueryDb;
use crate::stable_mir::CrateDef;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    AggregateKind, BasicBlock, BasicBlockIdx, Body, ConstOperand, Operand, Place, Rvalue,
    Statement, StatementKind, Terminator, TerminatorKind, VarDebugInfoContents,
};
use stable_mir::ty::{FnDef, MirConst, RigidTy, UintTy};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;

use super::TransformPass;

/// This pass will perform the following operations:
/// 1. Replace the body of `kani_register_loop_contract` by `kani::internal::run_contract_fn`
///    to invoke the closure.
///
/// 2. Transform loops with contracts from
///    ```ignore
///    bb_idx: {
///         loop_head_stmts
///         _v = kani_register_loop_contract(move args) -> [return: terminator_target];
///    }
///
///    ...
///    loop_body_blocks
///    ...
///
///    loop_latch_block: {
///         loop_latch_stmts
///         goto -> bb_idx;
///    }
///    ```
///    to blocks
///    ```ignore
///    bb_idx: {
///         loop_head_stmts
///         _v = true
///         goto -> terminator_target
///    }
///
///    ...
///    loop_body_blocks
///    ...
///
///    loop_latch_block: {
///         loop_latch_stmts
///         goto -> bb_new_loop_latch;
///    }
///
///    bb_new_loop_latch: {
///         loop_head_body
///         _v = kani_register_loop_contract(move args) -> [return: terminator_target];
///    }
///    ```
#[derive(Debug, Default)]
pub struct LoopContractPass {
    /// Cache KaniRunContract function used to implement contracts.
    run_contract_fn: Option<FnDef>,
    /// The map from original loop head to the new loop latch.
    /// We use this map to redirect all original loop latches to a new single loop latch.
    new_loop_latches: HashMap<usize, usize>,
}

impl TransformPass for LoopContractPass {
    /// The type of transformation that this pass implements.
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        query_db.args().unstable_features.contains(&"loop-contracts".to_string())
    }

    /// Run a transformation pass on the whole codegen unit.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(_func, args) => {
                if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_loop_contract"))
                {
                    // Replace the body of the register function with `run_contract_fn`'s.
                    let run = Instance::resolve(self.run_contract_fn.unwrap(), args).unwrap();
                    (true, run.body().unwrap())
                } else {
                    let mut new_body = MutableBody::from(body);
                    let mut contain_loop_contracts: bool = false;

                    // Visit basic blocks in control flow order (BFS).
                    let mut visited: HashSet<BasicBlockIdx> = HashSet::new();
                    let mut queue: VecDeque<BasicBlockIdx> = VecDeque::new();
                    // Visit blocks in loops only when there is no blocks in queue.
                    let mut loop_queue: VecDeque<BasicBlockIdx> = VecDeque::new();
                    queue.push_back(0);

                    while let Some(bb_idx) = queue.pop_front().or(loop_queue.pop_front()) {
                        visited.insert(bb_idx);

                        let terminator = new_body.blocks()[bb_idx].terminator.clone();

                        let is_loop_head = self.transform_bb(tcx, &mut new_body, bb_idx);
                        contain_loop_contracts |= is_loop_head;

                        // Add successors of the current basic blocks to
                        // the visiting queue.
                        for to_visit in terminator.successors() {
                            if !visited.contains(&to_visit) {
                                let target_queue =
                                    if is_loop_head { &mut loop_queue } else { &mut queue };
                                target_queue.push_back(to_visit);
                            }
                        }
                    }
                    (contain_loop_contracts, new_body.into())
                }
            }
            _ => {
                /* static variables case */
                (false, body)
            }
        }
    }
}

impl LoopContractPass {
    pub fn new(tcx: TyCtxt, unit: &CodegenUnit) -> LoopContractPass {
        if !unit.harnesses.is_empty() {
            let run_contract_fn = find_fn_def(tcx, "KaniRunLoopContract");
            assert!(run_contract_fn.is_some(), "Failed to find Kani run contract function");
            LoopContractPass { run_contract_fn, new_loop_latches: HashMap::new() }
        } else {
            // If reachability mode is PubFns or Tests, we just remove any contract logic.
            // Note that in this path there is no proof harness.
            LoopContractPass::default()
        }
    }

    /// Generate the body of loop head block by dropping all statements
    /// except for `StorageLive` and `StorageDead`.
    fn get_loop_head_block(&self, block: &BasicBlock) -> BasicBlock {
        let new_stmts: Vec<Statement> = block
            .statements
            .iter()
            .filter(|stmt| {
                matches!(stmt.kind, StatementKind::StorageLive(_) | StatementKind::StorageDead(_))
            })
            .map(|stmt| stmt.clone())
            .collect();
        return BasicBlock { statements: new_stmts, terminator: block.terminator.clone() };
    }

    fn is_supported_argument_of_closure(&self, rv: &Rvalue, body: &MutableBody) -> bool {
        let var_debug_info = &body.var_debug_info();
        matches!(rv, Rvalue::Ref(_, _, place) if
        var_debug_info.iter().any(|info|
            matches!(&info.value, VarDebugInfoContents::Place(debug_place) if *place == *debug_place)
        ))
    }

    fn transform_bb(&mut self, tcx: TyCtxt, new_body: &mut MutableBody, bb_idx: usize) -> bool {
        let terminator = new_body.blocks()[bb_idx].terminator.clone();
        let mut contain_loop_contracts = false;

        // Redirect loop latches to the new latches.
        if let TerminatorKind::Goto { target: terminator_target } = &terminator.kind {
            if self.new_loop_latches.contains_key(terminator_target) {
                new_body.replace_terminator(
                    &SourceInstruction::Terminator { bb: bb_idx },
                    Terminator {
                        kind: TerminatorKind::Goto {
                            target: self.new_loop_latches[terminator_target],
                        },
                        span: terminator.span,
                    },
                );
            }
        }

        // Transform loop heads with loop contracts.
        if let TerminatorKind::Call {
            func: terminator_func,
            args: terminator_args,
            destination: terminator_destination,
            target: terminator_target,
            unwind: terminator_unwind,
        } = &terminator.kind
        {
            // Get the function signature of the terminator call.
            let Some(RigidTy::FnDef(fn_def, ..)) = terminator_func
                .ty(new_body.locals())
                .ok()
                .map(|fn_ty| fn_ty.kind().rigid().unwrap().clone())
            else {
                return false;
            };

            // The basic blocks end with register functions are loop head blocks.
            if KaniAttributes::for_def_id(tcx, fn_def.def_id()).fn_marker()
                == Some(Symbol::intern("kani_register_loop_contract"))
                && matches!(&terminator_args[1], Operand::Constant(op) if op.const_.eval_target_usize().unwrap() == 0)
            {
                contain_loop_contracts = true;

                // Collect supported vars assigned in the block.
                // And check if all arguments of the closure is supported.
                let mut supported_vars: Vec<Place> = Vec::new();
                // All user variables are support
                supported_vars.extend(new_body.var_debug_info().iter().filter_map(|info| {
                    match &info.value {
                        VarDebugInfoContents::Place(debug_place) => Some(debug_place.clone()),
                        _ => None,
                    }
                }));

                // For each assignment in the loop head block,
                // if it assigns to the closure place, we check if all arguments are supported;
                // if it assigns to other places, we cache if the assigned places are supported.
                for stmt in &new_body.blocks()[bb_idx].statements {
                    if let StatementKind::Assign(place, rvalue) = &stmt.kind {
                        match rvalue {
                            Rvalue::Aggregate(AggregateKind::Closure(..), closure_args) => {
                                if closure_args.iter().any(|arg| !matches!(arg, Operand::Copy(arg_place) | Operand::Move(arg_place) if supported_vars.contains(arg_place))) {
                                    unreachable!(
                                            "The loop invariant contains unsupported variables. \
                                            Please report github.com/model-checking/kani/issues/new?template=bug_report.md"
                                        );
                                }
                            }
                            _ => {
                                if self.is_supported_argument_of_closure(rvalue, new_body) {
                                    supported_vars.push(place.clone());
                                }
                            }
                        }
                    }
                }

                // Replace the original loop head block
                // ```ignore
                // bb_idx: {
                //          loop_head_stmts
                //          _v = kani_register_loop_contract(move args) -> [return: terminator_target];
                // }
                // ```
                // with
                // ```ignore
                // bb_idx: {
                //          loop_head_stmts
                //          _v = true;
                //          goto -> terminator_target
                // }
                // ```
                new_body.assign_to(
                    terminator_destination.clone(),
                    Rvalue::Use(Operand::Constant(ConstOperand {
                        span: terminator.span,
                        user_ty: None,
                        const_: MirConst::from_bool(true),
                    })),
                    &mut SourceInstruction::Terminator { bb: bb_idx },
                    InsertPosition::Before,
                );
                let new_latch_block = self.get_loop_head_block(&new_body.blocks()[bb_idx]);

                // Insert a new basic block as the loop latch block, and later redirect
                // all latches to the new loop latch block.
                // -----
                // bb_new_loop_latch: {
                //    _v = kani_register_loop_contract(move args) -> [return: terminator_target];
                // }
                new_body.insert_bb(
                    new_latch_block,
                    &mut SourceInstruction::Terminator { bb: bb_idx },
                    InsertPosition::After,
                );
                // Update the argument `transformed` to 1 to avoid double transformation.
                let new_args = vec![
                    terminator_args[0].clone(),
                    Operand::Constant(ConstOperand {
                        span: terminator.span,
                        user_ty: None,
                        const_: MirConst::try_from_uint(1, UintTy::Usize).unwrap(),
                    }),
                ];
                new_body.replace_terminator(
                    &SourceInstruction::Terminator { bb: new_body.blocks().len() - 1 },
                    Terminator {
                        kind: TerminatorKind::Call {
                            func: terminator_func.clone(),
                            args: new_args,
                            destination: terminator_destination.clone(),
                            target: *terminator_target,
                            unwind: *terminator_unwind,
                        },
                        span: terminator.span,
                    },
                );
                new_body.replace_terminator(
                    &SourceInstruction::Terminator { bb: bb_idx },
                    Terminator {
                        kind: TerminatorKind::Goto { target: terminator_target.unwrap() },
                        span: terminator.span,
                    },
                );
                // Cache the new loop latch.
                self.new_loop_latches.insert(bb_idx, new_body.blocks().len() - 1);
            }
        }
        contain_loop_contracts
    }
}
