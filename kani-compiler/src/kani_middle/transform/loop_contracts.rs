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
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    AggregateKind, BasicBlock, BasicBlockIdx, Body, ConstOperand, Operand, Rvalue, Statement,
    StatementKind, SwitchTargets, Terminator, TerminatorKind, VarDebugInfoContents,
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

//Useful for debugging
#[allow(dead_code)]
pub fn print_stable_mir_body(body: &Body) {
    println!("=== MIR Body Start ===");
    // Print basic blocks
    for (idx, block) in body.blocks.iter().enumerate() {
        println!("bb{idx}: {{");
        // Print statements
        for stmt in &block.statements {
            println!(" {stmt:?}");
        }
        // Print terminator
        println!(" {:?}", block.terminator);
        println!("}}");
    }
    println!("=== MIR Body End ===");
}

//Useful for debugging
#[allow(dead_code)]
pub fn print_mutablebody(body: &MutableBody) {
    println!("=== MIR Body Start ===");
    // Print basic blocks
    for (idx, block) in body.blocks().iter().enumerate() {
        println!("bb{idx}: {{");
        // Print statements
        for stmt in &block.statements {
            println!(" {stmt:?}");
        }
        // Print terminator
        println!(" {:?}", block.terminator);
        println!("}}");
    }
    println!("=== MIR Body End ===");
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

    //Move all variables initiation using function-call inside the loop body to the loop-head
    fn move_storagelive_call_to_loophead(
        &self,
        body: &mut MutableBody,
        loop_head_map: &HashMap<usize, usize>,
        found_local_list: Vec<usize>,
    ) {
        let mut found_local_list = found_local_list;
        let localvars = self.get_user_defined_variables(body);
        let mut move_call_list: Vec<(usize, usize, Statement, Terminator)> = Vec::new();
        for (block_idx, block) in body.blocks().iter().enumerate() {
            if let Some(stmt) = block.statements.last() {
                match stmt.kind {
                    StatementKind::StorageLive(local)
                        if (localvars.contains(&local) && !found_local_list.contains(&local)) =>
                    {
                        let terminator = block.terminator.clone();
                        let terminatorkind = block.terminator.kind.clone();
                        if let TerminatorKind::Call { destination: dest, .. } = terminatorkind
                            && dest.local == local
                            && let Some(&closest_loop_head) = loop_head_map.get(&block_idx)
                        {
                            move_call_list.push((
                                closest_loop_head,
                                block_idx,
                                stmt.clone(),
                                terminator,
                            ));
                            found_local_list.push(local);
                        }
                    }
                    _ => (),
                }
            }
        }
        let mut current_loop_head = 0;
        move_call_list.sort_by_key(|(closest_loop_head, _, _, _)| *closest_loop_head);
        for (loop_head, block_idx, stmt, ter) in move_call_list.iter() {
            let remove_pos = body.blocks()[*block_idx].statements.len() - 1;
            body.remove_stmt(*block_idx, remove_pos);
            let mut terminator = ter.clone();
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
                    target: Some(*loop_head),
                    unwind: *terminator_unwind,
                };
            }
            let new_block = BasicBlock { statements: vec![stmt.clone()], terminator };
            if current_loop_head != *loop_head {
                body.insert_bb(
                    new_block,
                    &mut SourceInstruction::Terminator { bb: *loop_head },
                    InsertPosition::Before,
                );
                current_loop_head = *loop_head;
            } else {
                let current_block_id = body.blocks().len() - 1;
                body.insert_bb(
                    new_block,
                    &mut SourceInstruction::Terminator { bb: current_block_id },
                    InsertPosition::After,
                );
            }
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
        let loop_head_map = self.get_associated_loop_head_hashmap(&new_body, tcx);
        let found_local_list =
            self.move_storagelive_assign_to_loophead(&mut new_body, &loop_head_map);
        //print_mutablebody(&new_body);
        //assert!(1==0);
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
                            Rvalue::Ref(_,_,rplace) | Rvalue::CopyForDeref(rplace) => {
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
