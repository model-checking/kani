// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains code related to the MIR-to-MIR pass to enable loop contracts.
//!

use super::TransformPass;
use crate::kani_middle::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::kani_functions::KaniModel;
use crate::kani_middle::transform::TransformationType;
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_queries::QueryDb;
use crate::stable_mir::CrateDef;
use itertools::Itertools;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    AggregateKind, BasicBlock, BasicBlockIdx, Body, ConstOperand, Operand, Place, Rvalue,
    Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind, VarDebugInfoContents,
};
use stable_mir::ty::{FnDef, GenericArgKind, MirConst, RigidTy, TyKind, UintTy};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;

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
    ///
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
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        self.new_loop_latches = HashMap::new();
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(_func, args) => {
                if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_loop_contract"))
                {
                    // Replace the body of the register function with `run_contract_fn`'s.
                    let run = Instance::resolve(self.run_contract_fn.unwrap(), args).unwrap();
                    (true, run.body().unwrap())
                } else {
                    self.transform_body_with_loop(tcx, body)
                }
            }
            RigidTy::Closure(_, _) => self.transform_body_with_loop(tcx, body),
            _ => {
                /* static variables case */
                (false, body)
            }
        }
    }
}

impl LoopContractPass {
    pub fn new(_tcx: TyCtxt, queries: &QueryDb, unit: &CodegenUnit) -> LoopContractPass {
        if !unit.harnesses.is_empty() {
            let run_contract_fn =
                queries.kani_functions().get(&KaniModel::RunLoopContract.into()).copied();
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
            .cloned()
            .collect();
        BasicBlock { statements: new_stmts, terminator: block.terminator.clone() }
    }

    /// Remove `StorageDead closure_var` to avoid invariant closure becoming dead.
    fn make_invariant_closure_alive(&self, body: &mut MutableBody, bb_idx: usize) {
        let mut stmts = body.blocks()[bb_idx].statements.clone();
        if stmts.is_empty() || !matches!(stmts[0].kind, StatementKind::StorageDead(_)) {
            unreachable!(
                "The assumptions for loop-contracts transformation are violated by some other transformation. \
            Please report github.com/model-checking/kani/issues/new?template=bug_report.md"
            );
        }
        stmts.remove(0);
        body.replace_statements(&SourceInstruction::Terminator { bb: bb_idx }, stmts);
    }

    //Get all user defined variables in the function (not the compiler-generated ones)
    //Note that there might be user defined variables with the same user defined names,
    //but they are all have different MIR-generated names.
    fn get_user_defined_variables(&self, body: &MutableBody) -> Vec<usize> {
        let mut user_vars = Vec::new();

        // Iterate through all locals
        for (idx, _) in body.locals().iter().enumerate() {
            // Skip the return place (local 0)
            if idx == 0 {
                continue;
            }

            // Check if this is a user-defined variable (not a compiler temp)
            let is_user_defined = body.var_debug_info().iter().any(|info| {
            matches!(&info.value, VarDebugInfoContents::Place(place) if place.local == idx)
        });

            if is_user_defined {
                user_vars.push(idx);
            }
        }

        user_vars
    }

    // Get the list of tuples:
    // (firstpat, the block_id where firstpat get assigned, the corresponding indexpat, the block_id where indexpat get assigned)
    fn get_firstpats_and_indexingpats(
        &self,
        body: &MutableBody,
    ) -> Vec<(usize, usize, usize, usize)> {
        let mut firstpats_and_indexpats: Vec<(usize, usize, usize, usize)> = Vec::new();
        let mut current_firstpat = 0;
        let mut current_firstpat_pos = 0;
        for (blockid, block) in body.blocks().iter().enumerate() {
            if let TerminatorKind::Call {
                func: terminator_func,
                args: _,
                destination: dest,
                target: _,
                unwind: _,
            } = &block.terminator.kind
            {
                // Get the function signature of the terminator call.
                let Some(RigidTy::FnDef(fn_def, _)) = terminator_func
                    .ty(body.locals())
                    .ok()
                    .map(|fn_ty| fn_ty.kind().rigid().unwrap().clone())
                else {
                    continue;
                };
                // Check if the function is `kani::internal::run_contract_fn`.
                if fn_def.name().to_string() == "kani::KaniIter::first" {
                    current_firstpat = dest.local;
                    current_firstpat_pos = blockid;
                }
                if fn_def.name().to_string() == "kani::KaniIter::indexing" {
                    if current_firstpat != 0 {
                        firstpats_and_indexpats.push((
                            current_firstpat,
                            dest.local,
                            current_firstpat_pos,
                            blockid,
                        ));
                        current_firstpat = 0
                    }
                }
            }
        }
        firstpats_and_indexpats
    }

    // This Vec includes the user defined variables together with the tuple-typed variable
    // thst store the return of "kani::KaniIter::first" function
    fn get_storage_moving_variables(&self, body: &MutableBody) -> Vec<usize> {
        let first_index_list = self.get_firstpats_and_indexingpats(body);
        let mut moving_vars = self.get_user_defined_variables(body);
        for (firstvar, _, _, _) in first_index_list {
            if !moving_vars.contains(&firstvar) {
                moving_vars.push(firstvar);
            }
        }
        moving_vars
    }

    // Return the same Terminator with new destination
    fn terminator_of_new_destination(old: Terminator, new_destination_local: usize) -> Terminator {
        let mut terminator = old.clone();
        if let TerminatorKind::Call {
            func: terminator_func,
            args: terminator_args,
            destination: old_destination,
            target: terminator_target,
            unwind: terminator_unwind,
        } = &terminator.kind
        {
            let mut new_destination = old_destination.clone();
            new_destination.local = new_destination_local;
            terminator.kind = TerminatorKind::Call {
                func: terminator_func.clone(),
                args: terminator_args.clone(),
                destination: new_destination,
                target: terminator_target.clone(),
                unwind: *terminator_unwind,
            };
        }
        terminator
    }

    // Replace the "firstpat" vars with its corresponding "indexingpat" vars
    // See the comments in kani/library/kani_macros/src/sysroot/loop_contracts/mod.rs
    fn replace_firstpat_by_indexingpat(&self, body: &mut MutableBody) {
        let first_indexing_list = self.get_firstpats_and_indexingpats(body);
        for (firstvar, indexvar, first_blockid, index_blockid) in first_indexing_list {
            // Replace firstpat by indexpat in the destination of "kani::KaniIter::first" function call
            let old_terminator = body.blocks()[first_blockid].terminator.clone();
            let new_terminator = Self::terminator_of_new_destination(old_terminator, indexvar);
            body.replace_terminator(
                &SourceInstruction::Terminator { bb: first_blockid },
                new_terminator,
            );
            let span = body.blocks()[first_blockid].statements.first().unwrap().span.clone();
            // Add the StorageLive(indexpat) statement at the begining of the same block
            let storagelive_stmt = Statement { kind: StatementKind::StorageLive(indexvar), span };
            body.insert_stmt(
                storagelive_stmt,
                &mut SourceInstruction::Statement { idx: 0, bb: first_blockid },
                InsertPosition::Before,
            );

            // Remove the StorageLive(firstpat) statement in the same block if any
            let mut storageliveid = None;
            for (id, stmt) in body.blocks()[first_blockid].statements.iter().enumerate() {
                if let StatementKind::StorageLive(local) = &stmt.kind
                    && *local == firstvar
                {
                    storageliveid = Some(id)
                }
            }
            if let Some(id) = storageliveid {
                body.remove_stmt(first_blockid, id);
            }

            // Construct the HashMap of the firstpat projections with  indexpat projections
            let firstprj_stmts_copy = body.blocks()[first_blockid + 1].statements.clone();
            let indexprj_stmts_copy = body.blocks()[index_blockid + 1].statements.clone();
            let mut firstprj_indexprj: HashMap<usize, usize> = HashMap::new();
            firstprj_indexprj.insert(firstvar, indexvar);

            for fstmt in firstprj_stmts_copy.iter() {
                if let StatementKind::Assign(fprjplace, frval) = &fstmt.kind
                    && let Rvalue::Use(Operand::Copy(firstpatplace)) = frval
                    && firstpatplace.local == firstvar
                {
                    let firstprj = fprjplace.local;
                    for istmt in indexprj_stmts_copy.iter() {
                        if let StatementKind::Assign(iprjplace, irval) = &istmt.kind
                            && let Rvalue::Use(Operand::Copy(indexpatplace)) = irval
                            && indexpatplace.local == indexvar
                            && indexpatplace.projection == firstpatplace.projection
                        {
                            let indexprj = iprjplace.local;
                            firstprj_indexprj.insert(firstprj, indexprj);
                            break;
                        }
                    }
                }
            }

            // Replace the firstpat (and its projections) with indexpat (and its projections)
            // in the places where they may get involved, which includes, the comments in code.
            // First, in the block that  "kani::KaniIter::first" is called
            let mut new_stmts: Vec<Statement> = Vec::new();
            for stmt in firstprj_stmts_copy.iter() {
                let mut new_stmt = stmt.clone();

                match &stmt.kind {
                    // The StorageLive statements of the projections
                    // There might be some without StorageLive statements
                    // So we just remove them and add a new one for each indexpat projection later
                    StatementKind::StorageLive(local) => {
                        if firstprj_indexprj.get(&local).is_none() {
                            new_stmts.push(new_stmt);
                        }
                    }
                    // The assign statements of the projections
                    StatementKind::Assign(fprjplace, frval) => {
                        if let Some(indexprj) = firstprj_indexprj.get(&fprjplace.local)
                            && let Rvalue::Use(Operand::Copy(firstpatplace)) = frval
                        {
                            let storagelive_stmt = Statement {
                                kind: StatementKind::StorageLive(*indexprj),
                                span: stmt.span.clone(),
                            };
                            new_stmts.push(storagelive_stmt);
                            let mut indexpatplace = firstpatplace.clone();
                            indexpatplace.local = indexvar;
                            let newrval = Rvalue::Use(Operand::Copy(indexpatplace));
                            new_stmt.kind = StatementKind::Assign(
                                Place {
                                    local: *indexprj,
                                    projection: fprjplace.projection.clone(),
                                },
                                newrval,
                            );
                            new_stmts.push(new_stmt);
                        } else {
                            new_stmts.push(new_stmt)
                        }
                    }
                    _ => new_stmts.push(new_stmt),
                }
            }

            body.replace_statements(
                &SourceInstruction::Statement { idx: 0, bb: first_blockid + 1 },
                new_stmts,
            );

            // Second, in the loophead block right after that
            let loophead_stmts_copy = body.blocks()[first_blockid + 2].statements.clone();
            let mut new_loophead_stmts = Vec::new();
            if let StatementKind::Assign(_, Rvalue::Ref(_, _, Place { local: closurelocal, .. })) =
                loophead_stmts_copy.last().unwrap().kind
            {
                for stmt in loophead_stmts_copy.iter() {
                    // In the Operands of the loop invariant closure
                    if let StatementKind::Assign(lhs, Rvalue::Aggregate(aggrkind, operands)) =
                        &stmt.kind
                        && lhs.local == closurelocal
                    {
                        let mut new_operands = Vec::new();
                        for operand in operands.iter() {
                            if let Operand::Move(Place { local: operandlocal, projection: proj }) =
                                operand
                                && let Some(indexprj) = firstprj_indexprj.get(operandlocal)
                            {
                                let new_operand = Operand::Move(Place {
                                    local: *indexprj,
                                    projection: proj.clone(),
                                });
                                new_operands.push(new_operand);
                            } else if let Operand::Copy(Place {
                                local: operandlocal,
                                projection: proj,
                            }) = operand
                                && let Some(indexprj) = firstprj_indexprj.get(operandlocal)
                            {
                                let new_operand = Operand::Copy(Place {
                                    local: *indexprj,
                                    projection: proj.clone(),
                                });
                                new_operands.push(new_operand);
                            } else {
                                new_operands.push(operand.clone());
                            }
                        }
                        let new_rval = Rvalue::Aggregate(aggrkind.clone(), new_operands);
                        new_loophead_stmts.push(Statement {
                            kind: StatementKind::Assign(lhs.clone(), new_rval),
                            span: stmt.span.clone(),
                        });
                    } else if let StatementKind::Assign(
                        lhs,
                        Rvalue::Ref(region, borrowkind, Place { local: firstlocal, projection }),
                    ) = &stmt.kind
                         // In the borrow statements 
                        && let Some(indexlocal) = firstprj_indexprj.get(firstlocal)
                    {
                        let new_rval = Rvalue::Ref(
                            region.clone(),
                            borrowkind.clone(),
                            Place { local: *indexlocal, projection: projection.clone() },
                        );
                        new_loophead_stmts.push(Statement {
                            kind: StatementKind::Assign(lhs.clone(), new_rval),
                            span: stmt.span.clone(),
                        });
                    } else {
                        new_loophead_stmts.push(stmt.clone());
                    }
                }
            } else {
                panic!("not a loop head")
            }

            body.replace_statements(
                &SourceInstruction::Statement { idx: 0, bb: first_blockid + 2 },
                new_loophead_stmts,
            );

            // Remove the StorageDead statements of indexpat and its projections
            let mut new_blocks = Vec::new();
            for (block_id, block) in body.blocks().iter().enumerate() {
                let mut new_stmts = Vec::new();
                for stmt in block.statements.iter() {
                    match &stmt.kind {
                        StatementKind::StorageDead(local) => {
                            if !firstprj_indexprj.values().contains(local) {
                                new_stmts.push(stmt.clone())
                            }
                        }
                        StatementKind::StorageLive(local) => {
                            if !(firstprj_indexprj.values().contains(local)
                                && block_id == index_blockid + 1)
                            {
                                new_stmts.push(stmt.clone())
                            }
                        }
                        _ => new_stmts.push(stmt.clone()),
                    }
                }
                new_blocks.push((block_id, new_stmts));
            }

            for (block_id, stmts) in new_blocks {
                body.replace_statements(
                    &SourceInstruction::Statement { idx: 0, bb: block_id },
                    stmts,
                );
            }
        }
    }

    // Get all the kaniiter variables of for loops
    fn get_kaniiter_variables(&self, body: &MutableBody) -> Vec<usize> {
        let mut user_vars = Vec::new();

        // Iterate through all locals
        for (idx, _) in body.locals().iter().enumerate() {
            // Skip the return place (local 0)
            if idx == 0 {
                continue;
            }
            let is_user_defined = body.var_debug_info().iter().any(|info| {
                matches!(&info.value, VarDebugInfoContents::Place(place) if place.local == idx)
                    && info.name.contains("kaniiter")
                    && !info.name.contains("kaniiterlen")
            });
            if is_user_defined {
                user_vars.push(idx);
            }
        }

        user_vars
    }

    fn is_loop_head(&self, body: &MutableBody, tcx: TyCtxt, block_idx: usize) -> bool {
        let terminator = body.blocks()[block_idx].terminator.clone();
        if let TerminatorKind::Call {
            func: terminator_func,
            args: terminator_args,
            destination: _,
            target: _,
            unwind: _,
        } = &terminator.kind
        {
            // Get the function signature of the terminator call.
            let Some(RigidTy::FnDef(fn_def, _)) = terminator_func
                .ty(body.locals())
                .ok()
                .map(|fn_ty| fn_ty.kind().rigid().unwrap().clone())
            else {
                return false;
            };
            // The basic blocks end with register functions are loop head blocks.
            KaniAttributes::for_def_id(tcx, fn_def.def_id()).fn_marker()
                == Some(Symbol::intern("kani_register_loop_contract"))
                && matches!(&terminator_args[1], Operand::Constant(op) if op.const_.eval_target_usize().unwrap() == 0)
        } else {
            false
        }
    }

    //Get all loop-positions: (loop_head_id, loop_latch_id) in the body
    fn get_loop_positions(&self, body: &MutableBody, tcx: TyCtxt) -> Vec<(usize, usize)> {
        let mut loop_pos: Vec<(usize, usize)> = Vec::new();
        for (block_idx, _) in body.blocks().iter().enumerate() {
            if self.is_loop_head(body, tcx, block_idx) {
                let loop_latch_id = self.get_last_loop_latch_id(body, block_idx);
                loop_pos.push((block_idx, loop_latch_id));
            }
        }
        loop_pos
    }

    //Get the associated loop-head of a block_id
    fn get_associated_loop_head(
        &self,
        block_idx: usize,
        loop_positions: &Vec<(usize, usize)>,
    ) -> Option<usize> {
        let mut current_loop_head: Option<usize> = None;
        for (loop_head_idx, loop_latch_idx) in loop_positions {
            if block_idx > *loop_head_idx && block_idx <= *loop_latch_idx {
                current_loop_head = Some(*loop_head_idx);
            }
        }
        current_loop_head
    }

    //Create a Hashmap for a block_id and its associated loop-head
    fn get_associated_loop_head_hashmap(
        &self,
        body: &MutableBody,
        tcx: TyCtxt,
    ) -> HashMap<usize, usize> {
        let loop_positions = self.get_loop_positions(body, tcx);
        let mut loop_head_map: HashMap<usize, usize> = HashMap::new();
        for (block_idx, _) in body.blocks().iter().enumerate() {
            let loop_head = self.get_associated_loop_head(block_idx, &loop_positions);
            if let Some(loop_head) = loop_head {
                loop_head_map.insert(block_idx, loop_head);
            }
        }
        loop_head_map
    }

    ///In case of nested loop, if a variable is declared and initiated inside a loop body, and assigned inside an inner-loop,
    ///then CBMC cannot infer the assign clause for the inner-loop after the loop-contract transformation.
    //Move all variables initiation using assign inside the loop body to the loop-head
    fn move_storagelive_assign_to_loophead(
        &self,
        body: &mut MutableBody,
        loop_head_map: &HashMap<usize, usize>,
    ) -> Vec<usize> {
        let mut add_assign_list: Vec<(usize, Statement)> = Vec::new();
        let mut found_local_list: Vec<usize> = Vec::new();
        let localvars = self.get_user_defined_variables(body);
        let mut blocks_stmts: Vec<(usize, Vec<Statement>)> = Vec::new();
        for (block_idx, block) in body.blocks().iter().enumerate() {
            if loop_head_map.get(&block_idx).is_none() {
                blocks_stmts.push((block_idx, block.statements.clone()));
                continue;
            }
            let closest_loop_head = *loop_head_map.get(&block_idx).unwrap();
            let stmts_len = block.statements.len();
            let mut new_stmts: Vec<Statement> = Vec::new();
            let mut stmt_idx = 0;
            while stmt_idx < stmts_len {
                let stmt = block.statements[stmt_idx].clone();
                if stmt_idx + 1 >= stmts_len {
                    new_stmts.push(stmt.clone());
                    break;
                }
                match stmt.kind {
                    StatementKind::StorageLive(local)
                        if (localvars.contains(&local) && !found_local_list.contains(&local)) =>
                    {
                        let next_stmt = block.statements[stmt_idx + 1].clone();
                        //Case 1: StorageLive followed by an assign
                        if matches!(next_stmt.kind.clone(), StatementKind::Assign(lhs,_) if lhs.local == local)
                        {
                            found_local_list.push(local);
                            add_assign_list.push((closest_loop_head, stmt.clone()));
                            add_assign_list.push((closest_loop_head, next_stmt.clone()));
                            new_stmts.push(next_stmt.clone());
                            stmt_idx += 2;
                            continue;
                        }
                        //Case 2: for Clone(): StorageLive followed by an StorageLive of a temp var, an assign ref of the temp var,
                        //Then an assign of the current local, then a StorageDead of the temp var
                        if let StatementKind::StorageLive(temp_local) = next_stmt.kind.clone()
                            && let Some(third_stmt) = block.statements.get(stmt_idx + 2)
                            && let Some(fourth_stmt) = block.statements.get(stmt_idx + 3)
                            && let Some(fifth_stmt) = block.statements.get(stmt_idx + 4)
                            && matches!(third_stmt.kind.clone(), StatementKind::Assign(lhs, _) if lhs.local == temp_local)
                            && matches!(fourth_stmt.kind.clone(), StatementKind::Assign(lhs, _) if lhs.local == local)
                            && matches!(fifth_stmt.kind.clone(), StatementKind::StorageDead(dead_local) if dead_local == temp_local)
                        {
                            found_local_list.push(local);
                            add_assign_list.push((closest_loop_head, stmt.clone()));
                            add_assign_list.push((closest_loop_head, next_stmt.clone()));
                            add_assign_list.push((closest_loop_head, third_stmt.clone()));
                            add_assign_list.push((closest_loop_head, fourth_stmt.clone()));
                            add_assign_list.push((closest_loop_head, fifth_stmt.clone()));
                            new_stmts.push(next_stmt.clone());
                            new_stmts.push(third_stmt.clone());
                            new_stmts.push(fourth_stmt.clone());
                            new_stmts.push(fifth_stmt.clone());
                            stmt_idx += 5;
                            continue;
                        }
                    }
                    _ => (),
                }
                new_stmts.push(stmt.clone());
                stmt_idx += 1;
            }
            blocks_stmts.push((block_idx, new_stmts));
        }

        for (block_idx, new_stmts) in blocks_stmts {
            body.replace_statements(&SourceInstruction::Terminator { bb: block_idx }, new_stmts);
        }

        for (block_idx, stmt) in add_assign_list {
            body.insert_stmt(
                stmt,
                &mut SourceInstruction::Terminator { bb: block_idx },
                InsertPosition::Before,
            );
        }
        found_local_list
    }

    fn terminator_of_new_target(old: Terminator, new_target: usize) -> Terminator {
        let mut terminator = old.clone();
        if let TerminatorKind::Call {
            func: terminator_func,
            args: terminator_args,
            destination: terminator_destination,
            target: _,
            unwind: terminator_unwind,
        } = &terminator.kind
        {
            terminator.kind = TerminatorKind::Call {
                func: terminator_func.clone(),
                args: terminator_args.clone(),
                destination: terminator_destination.clone(),
                target: Some(new_target),
                unwind: *terminator_unwind,
            };
        }
        terminator
    }

    fn block_of_new_target(old: &BasicBlock, new_target: usize) -> BasicBlock {
        let mut new_block = old.clone();
        new_block.terminator = Self::terminator_of_new_target(old.terminator.clone(), new_target);
        new_block
    }

    // Insert a list of blocks consecutively between the loop head and its next block
    fn insert_blocks_from_loophead(
        body: &mut MutableBody,
        blocks: &Vec<BasicBlock>,
        loophead: usize,
    ) {
        for (i, block) in blocks.iter().enumerate() {
            if i == 0 {
                let modified_block = Self::block_of_new_target(block, loophead);
                body.insert_bb(
                    modified_block,
                    &mut SourceInstruction::Terminator { bb: loophead },
                    InsertPosition::Before,
                )
            } else {
                let modified_block = if i == blocks.len() - 1 {
                    Self::block_of_new_target(block, loophead)
                } else {
                    Self::block_of_new_target(block, body.blocks().len() + 1)
                };
                body.insert_bb(
                    modified_block,
                    &mut SourceInstruction::Terminator { bb: body.blocks().len() - 1 },
                    InsertPosition::After,
                );
            }
        }
    }

    // Insert a list of blocks consecutively at the end of the body then let the final one connect to the loop-head
    fn insert_blocks_from_at_bottom_connect_to_loophead(
        body: &mut MutableBody,
        blocks: &Vec<BasicBlock>,
        loophead: usize,
    ) {
        for (i, block) in blocks.iter().enumerate() {
            let modified_block = if i == blocks.len() - 1 {
                Self::block_of_new_target(block, loophead)
            } else {
                Self::block_of_new_target(block, body.blocks().len() + 1)
            };
            body.insert_bb(
                modified_block,
                &mut SourceInstruction::Terminator { bb: body.blocks().len() - 1 },
                InsertPosition::After,
            );
        }
    }

    //Move all variables initiation using function-call inside the loop body to the loop-head
    fn move_storagelive_call_to_loophead(
        &self,
        body: &mut MutableBody,
        loop_head_map: &HashMap<usize, usize>,
        found_local_list: Vec<usize>,
    ) {
        let mut found_local_list = found_local_list;
        let localvars = self.get_storage_moving_variables(body);
        let forloopvars = self.get_kaniiter_variables(body);
        let mut current_user_local = 0;
        let mut current_local_decl_blocks: Vec<BasicBlock> = Vec::new();
        let mut move_call_list: Vec<(usize, Vec<BasicBlock>)> = Vec::new();
        let mut kaniiter_blocks: Vec<usize> = Vec::new();
        for (block_idx, block) in body.blocks().iter().enumerate() {
            let mut decl_current_user_local = false;
            let mut storage_live_block_stmt: Vec<Statement> = Vec::new();
            if loop_head_map.get(&block_idx).is_none() {
                continue;
            }
            let closest_loop_head = *loop_head_map.get(&block_idx).unwrap();
            let terminator = block.terminator.clone();
            let terminatorkind = block.terminator.kind.clone();
            for stmt in block.statements.clone() {
                if let StatementKind::StorageLive(local) = stmt.kind
                    && (localvars.contains(&local) && !found_local_list.contains(&local))
                    && current_user_local == 0
                {
                    current_user_local = local;
                    found_local_list.push(local);
                    decl_current_user_local = true;
                }
                if decl_current_user_local {
                    storage_live_block_stmt.push(stmt.clone());
                }
            }

            if decl_current_user_local {
                let first_block = BasicBlock {
                    statements: storage_live_block_stmt.clone(),
                    terminator: terminator.clone(),
                };
                current_local_decl_blocks.push(first_block)
            } else if current_user_local != 0 {
                current_local_decl_blocks.push(block.clone());
            }

            if let TerminatorKind::Call { destination: dest, .. } = terminatorkind.clone()
                && dest.local == current_user_local
                && current_user_local != 0
            {
                move_call_list.push((closest_loop_head, current_local_decl_blocks.clone()));
                current_local_decl_blocks = Vec::new();
                current_user_local = 0;
            }

            if let TerminatorKind::Call { destination: dest, .. } = terminatorkind
                && forloopvars.contains(&dest.local)
            {
                kaniiter_blocks.push(block_idx);
            }
        }

        let mut current_loop_head = 0;
        move_call_list.sort_by_key(|(closest_loop_head, _)| *closest_loop_head);
        for (loophead, blocks) in move_call_list.iter() {
            if current_loop_head != *loophead {
                Self::insert_blocks_from_loophead(body, blocks, *loophead);
                current_loop_head = *loophead;
            } else {
                Self::insert_blocks_from_at_bottom_connect_to_loophead(body, blocks, *loophead);
            }
        }

        // For the performance benefits remove the re-assign statements of kaniiter variables
        // after adding the same one at loop head
        for block_idx in kaniiter_blocks {
            let span = body.blocks()[block_idx].terminator.span.clone();
            body.replace_terminator(
                &SourceInstruction::Terminator { bb: block_idx },
                Terminator { kind: TerminatorKind::Goto { target: block_idx + 1 }, span },
            );
        }
    }

    //Move all storagedead inside the loop body to the loop termination block
    fn move_storagedead(&self, body: &mut MutableBody, src_block_idx: usize, dst_block_idx: usize) {
        let localvars = self.get_user_defined_variables(body);
        let storagedead_stmts: Vec<_> = body.blocks()[src_block_idx]
            .clone()
            .statements
            .iter()
            .filter(
                |stmt| matches!(stmt.kind, StatementKind::StorageDead(x) if localvars.contains(&x)),
            )
            .cloned()
            .collect();
        let other_stmts: Vec<_> = body.blocks()[src_block_idx]
            .clone()
            .statements
            .iter()
            .filter(|stmt| !matches!(stmt.kind, StatementKind::StorageDead(x) if localvars.contains(&x)))
            .cloned()
            .collect();
        body.replace_statements(&SourceInstruction::Terminator { bb: src_block_idx }, other_stmts);
        let mut new_stmts = body.blocks()[dst_block_idx].statements.clone();
        let dst_block_stmt_kind: Vec<_> = new_stmts.iter().map(|st| st.kind.clone()).collect();
        for stmt in storagedead_stmts.iter() {
            if !dst_block_stmt_kind.contains(&stmt.kind) {
                new_stmts.push(stmt.clone())
            }
        }
        body.replace_statements(&SourceInstruction::Terminator { bb: dst_block_idx }, new_stmts);
    }

    //Get the associated final loop-latch-id of a loop-head-id
    fn get_last_loop_latch_id(&self, body: &MutableBody, loop_head_id: usize) -> usize {
        let mut loop_latch_id = loop_head_id;
        for (bb_idx, block) in body.blocks().iter().enumerate() {
            match block.terminator.kind {
                TerminatorKind::Goto { target }
                    if (target == loop_head_id && bb_idx > loop_head_id) =>
                {
                    loop_latch_id = bb_idx;
                }
                _ => (),
            }
        }
        loop_latch_id
    }

    //Get the all associated loop-latch-ids of a loop-head-id
    fn get_all_loop_latch_ids(&self, body: &MutableBody, loop_head_id: usize) -> Vec<usize> {
        let mut loop_latch_ids = Vec::new();
        for (bb_idx, block) in body.blocks().iter().enumerate() {
            match block.terminator.kind {
                TerminatorKind::Goto { target }
                    if (target == loop_head_id && bb_idx > loop_head_id) =>
                {
                    loop_latch_ids.push(bb_idx);
                }
                _ => (),
            }
        }
        loop_latch_ids
    }

    /// We only support closure arguments that are either `copy`` or `move`` of reference of user variables.
    fn is_supported_argument_of_closure(&self, rv: &Rvalue, body: &MutableBody) -> bool {
        let var_debug_info = &body.var_debug_info();
        matches!(rv, Rvalue::Ref(_, _, place) if
        var_debug_info.iter().any(|info|
            matches!(&info.value, VarDebugInfoContents::Place(debug_place) if *place == *debug_place)
        ))
    }

    /// This function transform the function body as described in fn transform.
    /// It is the core of fn transform, and is separated just to avoid code repetition.
    fn transform_body_with_loop(&mut self, tcx: TyCtxt, body: Body) -> (bool, Body) {
        let mut new_body = MutableBody::from(body);
        self.replace_firstpat_by_indexingpat(&mut new_body);
        let loop_head_map = self.get_associated_loop_head_hashmap(&new_body, tcx);
        let found_local_list =
            self.move_storagelive_assign_to_loophead(&mut new_body, &loop_head_map);
        let mut contain_loop_contracts: bool = false;

        // Visit basic blocks in control flow order (BFS).
        let mut visited: HashSet<BasicBlockIdx> = HashSet::new();
        let mut queue: VecDeque<BasicBlockIdx> = VecDeque::new();
        // Visit blocks in loops only when there is no blocks in queue.
        let mut loop_queue: VecDeque<BasicBlockIdx> = VecDeque::new();
        queue.push_back(0);

        while let Some(bb_idx) = queue.pop_front().or_else(|| loop_queue.pop_front()) {
            visited.insert(bb_idx);

            let terminator = new_body.blocks()[bb_idx].terminator.clone();

            let is_loop_head = self.transform_bb(tcx, &mut new_body, bb_idx);
            contain_loop_contracts |= is_loop_head;

            // Add successors of the current basic blocks to
            // the visiting queue.
            for to_visit in terminator.successors() {
                if !visited.contains(&to_visit) {
                    if is_loop_head {
                        loop_queue.push_back(to_visit);
                    } else {
                        queue.push_back(to_visit)
                    };
                }
            }
        }
        self.move_storagelive_call_to_loophead(&mut new_body, &loop_head_map, found_local_list);
        (contain_loop_contracts, new_body.into())
    }

    /// Transform loops with contracts from
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
    fn transform_bb(&mut self, tcx: TyCtxt, new_body: &mut MutableBody, bb_idx: usize) -> bool {
        let terminator = new_body.blocks()[bb_idx].terminator.clone();
        let mut contain_loop_contracts = false;

        // Redirect loop latches to the new latches.
        if let TerminatorKind::Goto { target: terminator_target } = &terminator.kind
            && self.new_loop_latches.contains_key(terminator_target)
        {
            new_body.replace_terminator(
                &SourceInstruction::Terminator { bb: bb_idx },
                Terminator {
                    kind: TerminatorKind::Goto { target: self.new_loop_latches[terminator_target] },
                    span: terminator.span,
                },
            );
        }

        if let TerminatorKind::SwitchInt { discr, targets } = &terminator.kind {
            let new_branches: Vec<_> = targets
                .branches()
                .map(|(a, b)| {
                    if self.new_loop_latches.contains_key(&b) {
                        (a, self.new_loop_latches[&b])
                    } else {
                        (a, b)
                    }
                })
                .collect();

            let new_otherwise = if self.new_loop_latches.contains_key(&targets.otherwise()) {
                self.new_loop_latches[&targets.otherwise()]
            } else {
                targets.otherwise()
            };

            let new_targets = SwitchTargets::new(new_branches, new_otherwise);
            new_body.replace_terminator(
                &SourceInstruction::Terminator { bb: bb_idx },
                Terminator {
                    kind: TerminatorKind::SwitchInt { discr: discr.clone(), targets: new_targets },
                    span: terminator.span,
                },
            );
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
            let Some(RigidTy::FnDef(fn_def, genarg)) = terminator_func
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
                let loop_termination_block_id = *new_body.blocks()[terminator_target.unwrap()]
                    .terminator
                    .clone()
                    .successors()
                    .first()
                    .unwrap();
                let loop_latch_ids = self.get_all_loop_latch_ids(new_body, bb_idx);
                for loop_latch_id in loop_latch_ids {
                    self.move_storagedead(new_body, loop_latch_id, loop_termination_block_id);
                }

                // Check if the MIR satisfy the assumptions of this transformation.
                if !new_body.blocks()[terminator_target.unwrap()].statements.is_empty()
                    || !matches!(
                        new_body.blocks()[terminator_target.unwrap()].terminator.kind,
                        TerminatorKind::SwitchInt { .. }
                    )
                {
                    unreachable!(
                        "The assumptions for loop-contracts transformation are violated by some other transformation. \
                    Please report github.com/model-checking/kani/issues/new?template=bug_report.md"
                    );
                }
                let GenericArgKind::Type(arg_ty) = genarg.0[0] else { return false };
                let TyKind::RigidTy(RigidTy::Closure(_, genarg)) = arg_ty.kind() else {
                    return false;
                };
                let GenericArgKind::Type(arg_ty) = genarg.0[2] else { return false };
                let TyKind::RigidTy(RigidTy::Tuple(args)) = arg_ty.kind() else { return false };
                // Check if the invariant involves any local variable
                if !args.is_empty() {
                    let ori_condition_bb_idx =
                        new_body.blocks()[terminator_target.unwrap()].terminator.successors()[1];
                    self.make_invariant_closure_alive(new_body, ori_condition_bb_idx);
                }

                contain_loop_contracts = true;

                // Collect supported vars assigned in the block.
                // And check if all arguments of the closure is supported.
                let mut supported_vars: Vec<usize> = Vec::new();
                // All user variables are support
                supported_vars.extend(new_body.var_debug_info().iter().filter_map(|info| {
                    match &info.value {
                        VarDebugInfoContents::Place(debug_place) => Some(debug_place.local),
                        _ => None,
                    }
                }));

                // For each assignment in the loop head block,
                // if it assigns to the closure place, we check if all arguments are supported;
                // if it assigns to other places, we cache if the assigned places are supported.
                for stmt in &new_body.blocks()[bb_idx].statements {
                    if let StatementKind::Assign(place, rvalue) = &stmt.kind {
                        match rvalue {
                            Rvalue::Ref(_,_,rplace) | Rvalue::CopyForDeref(rplace) | Rvalue::Use(Operand::Copy(rplace)) => {
                                if supported_vars.contains(&rplace.local) {
                                    supported_vars.push(place.local);
                                } }
                            Rvalue::Aggregate(AggregateKind::Closure(..), closure_args) => {
                                if closure_args.iter().any(|arg| !matches!(arg, Operand::Copy(arg_place) | Operand::Move(arg_place) if supported_vars.contains(&arg_place.local))) {
                                    unreachable!(
                                            "The loop invariant support only reference of user variables. The provided invariants contain unsupported dereference. \
                                            Please report github.com/model-checking/kani/issues/new?template=bug_report.md"
                                        );
                                }
                            }
                            _ => {
                                if self.is_supported_argument_of_closure(rvalue, new_body) {
                                    supported_vars.push(place.local);
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
