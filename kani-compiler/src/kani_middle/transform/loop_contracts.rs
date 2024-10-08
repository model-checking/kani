// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains code related to the MIR-to-MIR pass to enable loop contracts.
//!

use crate::kani_middle::KaniAttributes;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{BodyTransformation, CallGraph, TransformationResult};
use crate::kani_queries::QueryDb;
use crate::stable_mir::CrateDef;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use stable_mir::DefId;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::{
    BasicBlock, BasicBlockIdx, Body, ConstOperand, Operand, Rvalue, Statement, StatementKind,
    Terminator, TerminatorKind,
};
use stable_mir::ty::{FnDef, MirConst, RigidTy};
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use super::GlobalPass;

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
///         _v = kani_register_loop_contract(move args) -> [return: bb_idx];
///    }
///    ```
///
/// 3. Move the statements `loop_head_stmts` of loop head  with contracts to the body of register
///    functions for later codegen them as statement_expression in CBMC loop contracts.
#[derive(Debug, Default)]
pub struct LoopContractPass {
    /// Cache KaniRunContract function used to implement contracts.
    run_contract_fn: Option<FnDef>,
    /// The bb_idx of the new loop latch block.
    /// Keys are the original loop head.
    new_loop_latches: HashMap<usize, usize>,
    /// Statements of loop head with loop contracts.
    registered_stmts: HashMap<DefId, Vec<Statement>>,
    /// If loop contracts is enabled.
    loop_contracts_enabled: bool,
}

impl GlobalPass for LoopContractPass {
    fn is_enabled(&self, _query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Run a transformation pass on the whole codegen unit.
    fn transform(
        &mut self,
        tcx: TyCtxt,
        _call_graph: &CallGraph,
        _starting_items: &[MonoItem],
        instances: Vec<Instance>,
        transformer: &mut BodyTransformation,
    ) {
        if self.loop_contracts_enabled {
            // First transform functions with loop contracts.
            for instance in &instances {
                let body = instance.body().unwrap();
                let (modified, new_body) = self.transform_main_body(tcx, body, *instance);
                if modified {
                    transformer.cache.entry(*instance).and_modify(|transformation_result| {
                        *transformation_result = TransformationResult::Modified(new_body);
                    });
                }
            }

            // Move loop head statements with loop contracts to the corresponding register functions.
            for instance in &instances {
                let body = instance.body().unwrap();
                let (modified, new_body) = self.transform_register_body(tcx, body, *instance);
                if modified {
                    transformer.cache.entry(*instance).and_modify(|transformation_result| {
                        *transformation_result = TransformationResult::Modified(new_body);
                    });
                }
            }
        } else {
            for instance in &instances {
                let body = instance.body().unwrap();
                let (modified, new_body) =
                    self.remove_register_function_calls(tcx, body, *instance);
                if modified {
                    transformer.cache.entry(*instance).and_modify(|transformation_result| {
                        *transformation_result = TransformationResult::Modified(new_body);
                    });
                }
            }
        }
    }
}

impl LoopContractPass {
    pub fn new(tcx: TyCtxt, query_db: &QueryDb) -> LoopContractPass {
        let run_contract_fn = find_fn_def(tcx, "KaniRunContract");
        if run_contract_fn.is_some() {
            LoopContractPass {
                run_contract_fn,
                new_loop_latches: HashMap::new(),
                registered_stmts: HashMap::new(),
                loop_contracts_enabled: query_db
                    .args()
                    .unstable_features
                    .contains(&"loop-contracts".to_string()),
            }
        } else {
            LoopContractPass::default()
        }
    }

    fn remove_register_function_calls(
        &mut self,
        tcx: TyCtxt,
        body: Body,
        instance: Instance,
    ) -> (bool, Body) {
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(_func, _args) => {
                if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_loop_contract"))
                {
                    (false, body)
                } else {
                    let mut new_body = MutableBody::from(body);
                    let mut contain_loop_contracts: bool = false;

                    // Visit basic blocks in control flow order (BFS).
                    let mut visited: HashSet<BasicBlockIdx> = HashSet::new();
                    let mut queue: VecDeque<BasicBlockIdx> = VecDeque::new();
                    queue.push_back(0);

                    while let Some(bb_idx) = queue.pop_front() {
                        visited.insert(bb_idx);

                        let terminator = new_body.blocks()[bb_idx].terminator.clone();

                        if let TerminatorKind::Call {
                            func: terminator_func,
                            args: _,
                            destination: terminator_destination,
                            target: terminator_target,
                            unwind: _,
                        } = &terminator.kind
                        {
                            // Get the function signature of the terminator call.
                            let fn_kind = match terminator_func.ty(new_body.locals()) {
                                Ok(fn_ty) => fn_ty.kind(),
                                _ => continue,
                            };
                            let RigidTy::FnDef(fn_def, ..) = fn_kind.rigid().unwrap() else {
                                continue;
                            };

                            // The basic blocks end with register functions are loop head blocks.
                            if KaniAttributes::for_def_id(tcx, fn_def.def_id()).fn_marker()
                                == Some(Symbol::intern("kani_register_loop_contract"))
                            {
                                contain_loop_contracts = true;

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
                                //          _v = true;
                                //          goto -> terminator_target
                                // }
                                // ```
                                let new_loop_head_block_stmts: Vec<Statement> = vec![Statement {
                                    kind: StatementKind::Assign(
                                        terminator_destination.clone(),
                                        Rvalue::Use(Operand::Constant(ConstOperand {
                                            span: terminator.span,
                                            user_ty: None,
                                            const_: MirConst::from_bool(true),
                                        })),
                                    ),
                                    span: terminator.span,
                                }];

                                new_body.replace_statements(
                                    &SourceInstruction::Terminator { bb: bb_idx },
                                    new_loop_head_block_stmts,
                                );

                                new_body.replace_terminator(
                                    &SourceInstruction::Terminator { bb: bb_idx },
                                    Terminator {
                                        kind: TerminatorKind::Goto {
                                            target: terminator_target.unwrap(),
                                        },
                                        span: terminator.span,
                                    },
                                );
                            }
                        }

                        // Add successors of the current basic blocks to
                        // the visiting queue.
                        for to_visit in terminator.successors() {
                            if visited.contains(&to_visit) {
                                continue;
                            }
                            queue.push_back(to_visit);
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

    /// Transform bodies of loop contract register functions.
    /// 1. Replace the body of the register function with `run_contract_fn`'s.
    /// 2. Move statements `loop_head_stmts` of loop head  with contracts to the body of register functions
    ///    and make them unreachable. We will later codegen them when when codegen the calls to register functions.
    fn transform_register_body(
        &mut self,
        tcx: TyCtxt,
        body: Body,
        instance: Instance,
    ) -> (bool, Body) {
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(_func, args) => {
                if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_loop_contract"))
                    && self.registered_stmts.contains_key(&instance.def.def_id())
                {
                    // Replace the body of the register function with `run_contract_fn`'s.
                    let run = Instance::resolve(self.run_contract_fn.unwrap(), args).unwrap();

                    // Move the stmts `loop_head_stmts` of loop head  with contracts to the corresponding register functions.
                    let mut new_body = MutableBody::from(run.body().unwrap());
                    new_body.insert_bb(
                        BasicBlock {
                            statements: self.registered_stmts[&instance.def.def_id()].clone(),
                            terminator: Terminator {
                                kind: TerminatorKind::Goto { target: new_body.blocks().len() },
                                span: new_body.blocks()[0].terminator.span,
                            },
                        },
                        &mut SourceInstruction::Terminator { bb: 0 },
                        InsertPosition::Before,
                    );
                    new_body.replace_terminator(
                        &SourceInstruction::Terminator { bb: 0 },
                        Terminator {
                            kind: TerminatorKind::Goto { target: new_body.blocks().len() - 2 },
                            span: new_body.blocks()[0].terminator.span,
                        },
                    );
                    (true, new_body.into())
                } else {
                    (false, body)
                }
            }
            _ => {
                /* static variables case */
                (false, body)
            }
        }
    }

    /// Transform main function bodies with loop contracts.
    fn transform_main_body(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(_func, _args) => {
                if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_loop_contract"))
                {
                    // Register functions will be handled by `transform_register_body`.
                    (false, body)
                } else {
                    let mut new_body = MutableBody::from(body);
                    let mut contain_loop_contracts: bool = false;

                    // Visit basic blocks in control flow order (BFS).
                    let mut visited: HashSet<BasicBlockIdx> = HashSet::new();
                    let mut queue: VecDeque<BasicBlockIdx> = VecDeque::new();
                    queue.push_back(0);

                    while let Some(bb_idx) = queue.pop_front() {
                        visited.insert(bb_idx);

                        let terminator = new_body.blocks()[bb_idx].terminator.clone();

                        // Redirect loop latches to the new latches.
                        if let TerminatorKind::Goto { target: terminator_target } = &terminator.kind
                        {
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

                        if let TerminatorKind::Call {
                            func: terminator_func,
                            args: terminator_args,
                            destination: terminator_destination,
                            target: terminator_target,
                            unwind: terminator_unwind,
                        } = &terminator.kind
                        {
                            // Get the function signature of the terminator call.
                            let fn_kind = match terminator_func.ty(new_body.locals()) {
                                Ok(fn_ty) => fn_ty.kind(),
                                _ => continue,
                            };
                            let RigidTy::FnDef(fn_def, ..) = fn_kind.rigid().unwrap() else {
                                continue;
                            };

                            // The basic blocks end with register functions are loop head blocks.
                            if KaniAttributes::for_def_id(tcx, fn_def.def_id()).fn_marker()
                                == Some(Symbol::intern("kani_register_loop_contract"))
                            {
                                contain_loop_contracts = true;

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
                                //          _v = true;
                                //          goto -> terminator_target
                                // }
                                // ```
                                let new_loop_head_block_stmts: Vec<Statement> = vec![Statement {
                                    kind: StatementKind::Assign(
                                        terminator_destination.clone(),
                                        Rvalue::Use(Operand::Constant(ConstOperand {
                                            span: terminator.span,
                                            user_ty: None,
                                            const_: MirConst::from_bool(true),
                                        })),
                                    ),
                                    span: terminator.span,
                                }];

                                self.registered_stmts.insert(
                                    fn_def.def_id(),
                                    new_body.blocks()[bb_idx].statements.clone(),
                                );

                                new_body.replace_statements(
                                    &SourceInstruction::Terminator { bb: bb_idx },
                                    new_loop_head_block_stmts,
                                );

                                new_body.replace_terminator(
                                    &SourceInstruction::Terminator { bb: bb_idx },
                                    Terminator {
                                        kind: TerminatorKind::Goto { target: bb_idx },
                                        span: terminator.span,
                                    },
                                );

                                // Insert a new basic block as the loop latch block, and later redirect
                                // all latches to the new loop latch block.
                                // -----
                                // bb_new_loop_latch: {
                                //    _v = kani_register_loop_contract(move args) -> [return: bb_idx];
                                // }
                                new_body.insert_terminator(
                                    &mut SourceInstruction::Terminator { bb: bb_idx },
                                    InsertPosition::After,
                                    Terminator {
                                        kind: TerminatorKind::Call {
                                            func: terminator_func.clone(),
                                            args: terminator_args.clone(),
                                            destination: terminator_destination.clone(),
                                            target: *terminator_target,
                                            unwind: *terminator_unwind,
                                        },
                                        span: terminator.span,
                                    },
                                );
                                self.new_loop_latches.insert(bb_idx, new_body.blocks().len() - 1);
                            }
                        }

                        // Add successors of the current basic blocks to
                        // the visiting queue.
                        for to_visit in terminator.successors() {
                            if visited.contains(&to_visit) {
                                continue;
                            }
                            queue.push_back(to_visit);
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
