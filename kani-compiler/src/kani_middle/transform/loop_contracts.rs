// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains code related to the MIR-to-MIR pass to enable loop contracts.
//!

use super::TransformPass;
use crate::kani_middle::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::kani_functions::KaniModel;
use crate::kani_middle::transform::TransformationType;
use crate::kani_middle::transform::body::{
    InsertPosition, MutMirVisitor, MutableBody, SourceInstruction, synthetic_source_info,
};
use crate::kani_queries::QueryDb;
use crate::rustc_public::CrateDef;
use itertools::Itertools;
use rustc_middle::ty::TyCtxt;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    AggregateKind, BasicBlock, BasicBlockIdx, Body, ConstOperand, Local, Operand, Place,
    ProjectionElem, Rvalue, Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind,
    VarDebugInfoContents, WithRetag,
};
use rustc_public::rustc_internal;
use rustc_public::ty::{FnDef, GenericArgKind, MirConst, RigidTy, TyKind, UintTy};
use rustc_span::Symbol;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;

#[derive(Debug, Default, Clone)]
pub struct LoopContractPass {
    /// Cache KaniRunContract function used to implement contracts.
    run_contract_fn: Option<FnDef>,
    /// The map from original loop head to the new loop latch.
    /// We use this map to redirect all original loop latches to a new single loop latch.
    new_loop_latches: HashMap<usize, usize>,
    /// The map from original loop head to the locals that this transformation (together with
    /// the rewrite of the loop by `#[kani::loop_invariant]`) makes live across iterations of
    /// that loop, although they are not visible to the user at the loop contract: the
    /// pattern of a `for` loop, and the variables declared in the loop body (and the
    /// temporaries) whose initialization is copied to the loop head.
    /// See [LoopContractPass::add_generated_loop_modifies].
    generated_loop_locals: HashMap<usize, HashSet<usize>>,
    /// The locals to add to the loop modifies clause of each loop of the last transformed body,
    /// by the instance of the register function that the loop latch calls. Taken by
    /// [BodyTransformation](super::BodyTransformation) after each transformation, which keys
    /// them by the instance of the body as well.
    generated_loop_modifies: Vec<(Instance, Vec<Local>)>,
    /// The `kani_loop_modifies` binding of the loop modifies clause of each loop of the last
    /// transformed body, with the instance of the register function of that loop. Taken by
    /// [BodyTransformation](super::BodyTransformation) after each transformation, which keys
    /// them by the instance of the body as well. See [LoopContractPass::find_loop_modifies_bindings].
    loop_modifies_bindings: Vec<(Local, Instance)>,
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
        self.generated_loop_locals = HashMap::new();
        self.generated_loop_modifies = Vec::new();
        self.loop_modifies_bindings = Vec::new();
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

    fn take_generated_loop_modifies(&mut self) -> Vec<(Instance, Vec<Local>)> {
        std::mem::take(&mut self.generated_loop_modifies)
    }

    fn take_loop_modifies_bindings(&mut self) -> Vec<(Local, Instance)> {
        std::mem::take(&mut self.loop_modifies_bindings)
    }
}

/// Replaces the uses of the locals of the first pattern of a `for` loop by the corresponding
/// locals of the nth pattern, see [LoopContractPass::replace_first_pat_by_nth_pat].
/// The assigned places of statements and the dropped places are not changed: renaming the drop
/// of the first pattern at the end of its scope would drop the value of the nth pattern, which
/// the loop body has already dropped.
struct FirstPatRenamer<'a> {
    firstprj_nthprj: &'a HashMap<usize, usize>,
}

impl FirstPatRenamer<'_> {
    fn rename(&self, place: &mut Place) {
        if let Some(nthlocal) = self.firstprj_nthprj.get(&place.local) {
            place.local = *nthlocal;
        }
        for elem in place.projection.iter_mut() {
            if let ProjectionElem::Index(local) = elem
                && let Some(nthlocal) = self.firstprj_nthprj.get(local)
            {
                *local = *nthlocal;
            }
        }
    }
}

impl MutMirVisitor for FirstPatRenamer<'_> {
    fn visit_operand(&mut self, operand: &mut Operand) {
        if let Operand::Copy(place) | Operand::Move(place) = operand {
            self.rename(place);
        }
    }

    fn visit_rvalue(&mut self, rvalue: &mut Rvalue) {
        match rvalue {
            Rvalue::Ref(_, _, place)
            | Rvalue::AddressOf(_, place)
            | Rvalue::CopyForDeref(place)
            | Rvalue::Discriminant(place)
            | Rvalue::Len(place) => self.rename(place),
            _ => {}
        }
        self.super_rvalue(rvalue)
    }
}

impl LoopContractPass {
    pub fn new(_tcx: TyCtxt, queries: &QueryDb, unit: &CodegenUnit) -> LoopContractPass {
        if !unit.harnesses.is_empty() {
            let run_contract_fn =
                queries.kani_functions().get(&KaniModel::RunLoopContract.into()).copied();
            assert!(run_contract_fn.is_some(), "Failed to find Kani run contract function");
            LoopContractPass {
                run_contract_fn,
                new_loop_latches: HashMap::new(),
                generated_loop_locals: HashMap::new(),
                generated_loop_modifies: Vec::new(),
                loop_modifies_bindings: Vec::new(),
            }
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
    // (firstpat, the block_id where firstpat get assigned, the corresponding nthpat, the block_id where nthpat get assigned)
    fn get_first_pats_and_nth_pats(&self, body: &MutableBody) -> Vec<(usize, usize, usize, usize)> {
        let mut first_pats_and_nth_pats: Vec<(usize, usize, usize, usize)> = Vec::new();
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
                if fn_def.name() == "kani::KaniIter::first" {
                    current_firstpat = dest.local;
                    current_firstpat_pos = blockid;
                }
                if fn_def.name() == "kani::KaniIter::nth" && current_firstpat != 0 {
                    first_pats_and_nth_pats.push((
                        current_firstpat,
                        dest.local,
                        current_firstpat_pos,
                        blockid,
                    ));
                    current_firstpat = 0
                }
            }
        }
        first_pats_and_nth_pats
    }

    // This Vec includes the user defined variables together with the tuple-typed variable
    // thst store the return of "kani::KaniIter::first" function
    fn get_storage_moving_variables(&self, body: &MutableBody) -> Vec<usize> {
        let first_nth_list = self.get_first_pats_and_nth_pats(body);
        let mut moving_vars = self.get_user_defined_variables(body);
        for (firstvar, _, _, _) in first_nth_list {
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
                target: *terminator_target,
                unwind: *terminator_unwind,
            };
        }
        terminator
    }

    // Replace the "firstpat" vars with its corresponding "nthpat" vars
    // See the comments in kani/library/kani_macros/src/sysroot/loop_contracts/mod.rs
    // Return `false` if the loop head of one of these `for` loops could not be found. An error
    // has been emitted in that case, and the body must not be transformed further.
    fn replace_first_pat_by_nth_pat(&mut self, tcx: TyCtxt, body: &mut MutableBody) -> bool {
        let first_nth_list = self.get_first_pats_and_nth_pats(body);
        for (firstvar, nthvar, first_blockid, nth_blockid) in first_nth_list {
            let span = body.blocks()[first_blockid].terminator.source_info.span;
            // The blocks that the calls to "kani::KaniIter::first" and "kani::KaniIter::nth" return
            // to, which destructure the pattern.
            let (Some(first_target), Some(nth_target)) =
                (Self::call_target(body, first_blockid), Self::call_target(body, nth_blockid))
            else {
                Self::emit_for_loop_head_error(tcx, span);
                return false;
            };
            // Find the loop head and the blocks between the "kani::KaniIter::first" call and the
            // loop head, before changing the body.
            let Some((loop_head, blocks_before_head)) =
                self.find_for_loop_head(body, tcx, first_target, nth_blockid)
            else {
                Self::emit_for_loop_head_error(tcx, span);
                return false;
            };
            // The loop head ends with a borrow of the loop invariant closure.
            if !matches!(
                body.blocks()[loop_head].statements.last().map(|s| &s.kind),
                Some(StatementKind::Assign(_, Rvalue::Ref(..)))
            ) {
                Self::emit_for_loop_head_error(tcx, span);
                return false;
            }

            // Replace firstpat by nthpat in the destination of "kani::KaniIter::first" function call
            let old_terminator = body.blocks()[first_blockid].terminator.clone();
            let new_terminator = Self::terminator_of_new_destination(old_terminator, nthvar);
            body.replace_terminator(
                &SourceInstruction::Terminator { bb: first_blockid },
                new_terminator,
            );
            // Add the StorageLive(nthpat) statement at the begining of the same block
            let storagelive_stmt = Statement {
                kind: StatementKind::StorageLive(nthvar),
                source_info: synthetic_source_info(span),
            };
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

            // Construct the HashMap of the firstpat projections with  nthpat projections
            let firstprj_stmts_copy = body.blocks()[first_target].statements.clone();
            let nthprj_stmts_copy = body.blocks()[nth_target].statements.clone();
            let mut firstprj_nthprj: HashMap<usize, usize> = HashMap::new();
            firstprj_nthprj.insert(firstvar, nthvar);

            // Only user variables (pattern bindings) are paired up with their nthpat
            // counterparts. Compiler-generated temporaries (e.g. the deref temporaries
            // that destructuring patterns like `&x` introduce) must not be paired:
            // their assignments are kept in place (with the rvalue redirected to
            // nthvar) so that later statements reading them remain well-defined.
            // Note: before rust-lang/rust#145513 such temporaries were assigned via
            // `Rvalue::CopyForDeref` and thus never matched the `Rvalue::Use` pattern
            // below; nowadays they are plain `Rvalue::Use(Operand::Copy)` assignments.
            let user_vars = self.get_user_defined_variables(body);
            for fstmt in firstprj_stmts_copy.iter() {
                if let StatementKind::Assign(fprjplace, frval) = &fstmt.kind
                    && let Rvalue::Use(Operand::Copy(firstpatplace), _) = frval
                    && firstpatplace.local == firstvar
                    && user_vars.contains(&fprjplace.local)
                {
                    let firstprj = fprjplace.local;
                    for istmt in nthprj_stmts_copy.iter() {
                        if let StatementKind::Assign(iprjplace, irval) = &istmt.kind
                            && let Rvalue::Use(Operand::Copy(nthpatplace), _) = irval
                            && nthpatplace.local == nthvar
                            && nthpatplace.projection == firstpatplace.projection
                        {
                            let nthprj = iprjplace.local;
                            firstprj_nthprj.insert(firstprj, nthprj);
                            break;
                        }
                    }
                }
            }

            // Replace the firstpat (and its projections) with nthpat (and its projections)
            // in the places where they may get involved, which includes, the comments in code.
            // First, in the block that  "kani::KaniIter::first" is called
            let mut new_stmts: Vec<Statement> = Vec::new();
            for stmt in firstprj_stmts_copy.iter() {
                let mut new_stmt = stmt.clone();

                match &stmt.kind {
                    // The StorageLive statements of the projections
                    // There might be some without StorageLive statements
                    // So we just remove them and add a new one for each nthpat projection later
                    StatementKind::StorageLive(local) => {
                        if !firstprj_nthprj.contains_key(local) {
                            new_stmts.push(new_stmt);
                        }
                    }
                    // The assign statements of the projections
                    StatementKind::Assign(fprjplace, frval) => {
                        match frval {
                            Rvalue::Use(Operand::Copy(firstpatplace), _) => {
                                if firstpatplace.local == firstvar {
                                    let mut nthpatplace = firstpatplace.clone();
                                    nthpatplace.local = nthvar;
                                    let newrval =
                                        Rvalue::Use(Operand::Copy(nthpatplace), WithRetag::No);
                                    if let Some(nthprj) = firstprj_nthprj.get(&fprjplace.local) {
                                        // A user pattern binding: redirect the assignment to
                                        // the corresponding nthpat projection variable.
                                        new_stmt.kind = StatementKind::Assign(
                                            Place {
                                                local: *nthprj,
                                                projection: fprjplace.projection.clone(),
                                            },
                                            newrval,
                                        );
                                    } else {
                                        // A compiler-generated temporary (e.g. a deref
                                        // temporary, `Rvalue::CopyForDeref` before
                                        // rust-lang/rust#145513): keep the assigned place and
                                        // only redirect the rvalue to read from nthvar, so
                                        // that following statements dereferencing the
                                        // temporary keep working.
                                        new_stmt.kind =
                                            StatementKind::Assign(fprjplace.clone(), newrval);
                                    }
                                }
                            }
                            Rvalue::CopyForDeref(firstpatplace) => {
                                if firstpatplace.local == firstvar {
                                    let mut nthpatplace = firstpatplace.clone();
                                    nthpatplace.local = nthvar;
                                    let newrval = Rvalue::CopyForDeref(nthpatplace);
                                    new_stmt.kind =
                                        StatementKind::Assign(fprjplace.clone(), newrval);
                                }
                            }
                            _ => {
                                if let Some(nthprj) = firstprj_nthprj.get(&fprjplace.local) {
                                    new_stmt.kind = StatementKind::Assign(
                                        Place {
                                            local: *nthprj,
                                            projection: fprjplace.projection.clone(),
                                        },
                                        frval.clone(),
                                    )
                                }
                            }
                        }
                        new_stmts.push(new_stmt);
                    }
                    _ => new_stmts.push(new_stmt),
                }
            }

            body.replace_statements(
                &SourceInstruction::Statement { idx: 0, bb: first_target },
                new_stmts,
            );

            // Second, in the blocks between that block and the loop head (e.g., the `on_entry`
            // variables, or the clauses of a `#[kani::loop_modifies]` or `#[kani::loop_decreases]`
            // written after `#[kani::loop_invariant]`), and in the loop head itself, which
            // creates the loop invariant closure.
            for block in blocks_before_head.into_iter().chain(std::iter::once(loop_head)) {
                Self::redirect_first_pat_uses(body, block, &firstprj_nthprj);
            }

            // Remove the StorageDead statements of nthpat and its projections
            let mut new_blocks = Vec::new();
            for (block_id, block) in body.blocks().iter().enumerate() {
                let mut new_stmts = Vec::new();
                for stmt in block.statements.iter() {
                    match &stmt.kind {
                        StatementKind::StorageDead(local) => {
                            if !firstprj_nthprj.values().contains(local) {
                                new_stmts.push(stmt.clone())
                            }
                        }
                        StatementKind::StorageLive(local) => {
                            if !(firstprj_nthprj.values().contains(local) && block_id == nth_target)
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

            // The nthpat and its projections are assigned both before the loop and in each
            // iteration.
            self.generated_loop_locals
                .entry(loop_head)
                .or_default()
                .extend(firstprj_nthprj.values().copied());
        }
        true
    }

    /// The block that the call terminating `block` returns to, if `block` ends with a call.
    fn call_target(body: &MutableBody, block: usize) -> Option<usize> {
        match &body.blocks()[block].terminator.kind {
            TerminatorKind::Call { target, .. } => *target,
            _ => None,
        }
    }

    fn emit_for_loop_head_error(tcx: TyCtxt, span: rustc_public::ty::Span) {
        tcx.dcx()
            .struct_span_err(
                rustc_internal::internal(tcx, span),
                "Kani could not find the loop head of this `for` loop with a loop contract",
            )
            .with_note(
                "this can happen when an expression of the loop contract, \
                 e.g. in `#[kani::loop_modifies]`, diverges",
            )
            .emit();
    }

    /// Collect the blocks reachable from `starts` without going through a loop head (a block
    /// that ends with a call to the register function of a loop contract).
    /// Return the visited blocks that are not loop heads, and the loop heads that were reached,
    /// both in breadth-first order.
    fn reachable_before_loop_heads(
        &self,
        body: &MutableBody,
        tcx: TyCtxt,
        starts: Vec<usize>,
    ) -> (Vec<usize>, Vec<usize>) {
        let mut visited: HashSet<usize> = starts.iter().copied().collect();
        let mut queue: VecDeque<usize> = starts.into_iter().collect();
        let mut blocks = Vec::new();
        let mut loop_heads = Vec::new();
        while let Some(block) = queue.pop_front() {
            if self.is_loop_head(body, tcx, block) {
                loop_heads.push(block);
                continue;
            }
            blocks.push(block);
            for succ in body.blocks()[block].terminator.successors() {
                if visited.insert(succ) {
                    queue.push_back(succ);
                }
            }
        }
        (blocks, loop_heads)
    }

    /// Find the loop head of the `for` loop whose pattern is first assigned by the call to
    /// "kani::KaniIter::first" that returns to `first_target`, and whose body calls
    /// "kani::KaniIter::nth" in `nth_blockid`.
    ///
    /// The loop head is not necessarily right after `first_target`: the `for` loop rewrite (see
    /// library/kani_macros/src/sysroot/loop_contracts/mod.rs) evaluates the `on_entry` variables,
    /// the `prev` variables and the first iteration, and the clauses of the attributes that
    /// follow `#[kani::loop_invariant]` (e.g., `#[kani::loop_modifies(&a[i])]`, whose bounds
    /// check needs a block of its own) between the two.
    /// Return the loop head and the blocks visited before reaching it, or `None` if no loop head
    /// can be reached from `first_target`.
    fn find_for_loop_head(
        &self,
        body: &MutableBody,
        tcx: TyCtxt,
        first_target: usize,
        nth_blockid: usize,
    ) -> Option<(usize, Vec<usize>)> {
        let (blocks_before_head, loop_heads) =
            self.reachable_before_loop_heads(body, tcx, vec![first_target]);
        // With `prev`, other loops may be reachable as well (after the loop, through the branch
        // that does not enter the loop). The loop head of this `for` loop is the one whose body
        // calls "kani::KaniIter::nth".
        let loop_head = loop_heads.into_iter().find(|loop_head| {
            let successors = body.blocks()[*loop_head].terminator.successors();
            self.reachable_before_loop_heads(body, tcx, successors).0.contains(&nth_blockid)
        })?;
        Some((loop_head, blocks_before_head))
    }

    /// Replace the first pattern (and its projections) by the nth pattern (and its projections)
    /// in the operands, borrows and index projections of `block`.
    fn redirect_first_pat_uses(
        body: &mut MutableBody,
        block: usize,
        firstprj_nthprj: &HashMap<usize, usize>,
    ) {
        let mut new_block = body.blocks()[block].clone();
        FirstPatRenamer { firstprj_nthprj }.visit_basic_block(&mut new_block);
        body.replace_statements(
            &SourceInstruction::Statement { idx: 0, bb: block },
            new_block.statements,
        );
        body.replace_terminator(&SourceInstruction::Terminator { bb: block }, new_block.terminator);
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
                    && !info.name.contains("kani_iter_len")
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
        &mut self,
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
                            self.record_generated_loop_local(closest_loop_head, local);
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
                            self.record_generated_loop_local(closest_loop_head, local);
                            if temp_local > body.arg_count() && !localvars.contains(&temp_local) {
                                self.record_generated_loop_local(closest_loop_head, temp_local);
                            }
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

    /// Record that `local` is made live across the iterations of the loop with head `loop_head`
    /// by this transformation, see [LoopContractPass::generated_loop_locals].
    fn record_generated_loop_local(&mut self, loop_head: usize, local: usize) {
        self.generated_loop_locals.entry(loop_head).or_default().insert(local);
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
    fn insert_blocks_from_loophead(body: &mut MutableBody, blocks: &[BasicBlock], loophead: usize) {
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
        blocks: &[BasicBlock],
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
        &mut self,
        body: &mut MutableBody,
        loop_head_map: &HashMap<usize, usize>,
        found_local_list: Vec<usize>,
    ) {
        let mut found_local_list = found_local_list;
        let localvars = self.get_storage_moving_variables(body);
        let forloopvars = self.get_kaniiter_variables(body);
        let mut current_user_local = 0;
        // The loop head of the block that declares `current_user_local`.
        let mut current_user_local_loop_head = 0;
        let mut current_local_decl_blocks: Vec<BasicBlock> = Vec::new();
        let mut move_call_list: Vec<(usize, Vec<BasicBlock>)> = Vec::new();
        let mut moved_locals: Vec<(usize, usize, Vec<BasicBlock>)> = Vec::new();
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
                    current_user_local_loop_head = closest_loop_head;
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
                // Only a local that is declared in the body of this loop only exists for the
                // loop (a local declared in an outer loop, e.g. `let x;`, can be named in the
                // clause of this loop).
                if current_user_local_loop_head == closest_loop_head {
                    moved_locals.push((
                        closest_loop_head,
                        current_user_local,
                        current_local_decl_blocks.clone(),
                    ));
                }
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

        // The moved user variables, and the temporaries assigned by the moved blocks, are now
        // assigned at the loop head as well as in the loop body.
        let user_vars = self.get_user_defined_variables(body);
        for (loophead, user_local, blocks) in moved_locals {
            self.record_generated_loop_local(loophead, user_local);
            for block in &blocks {
                let assigned = block
                    .statements
                    .iter()
                    .filter_map(|stmt| match &stmt.kind {
                        StatementKind::Assign(place, _) => Some(place.local),
                        _ => None,
                    })
                    .chain(match &block.terminator.kind {
                        TerminatorKind::Call { destination, .. } => Some(destination.local),
                        _ => None,
                    });
                for local in assigned {
                    // Temporaries only, not the return place or the arguments.
                    if local > body.arg_count() && !user_vars.contains(&local) {
                        self.record_generated_loop_local(loophead, local);
                    }
                }
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
            let span = body.blocks()[block_idx].terminator.source_info.span;
            body.replace_terminator(
                &SourceInstruction::Terminator { bb: block_idx },
                Terminator {
                    kind: TerminatorKind::Goto { target: block_idx + 1 },
                    source_info: synthetic_source_info(span),
                },
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
        if !self.replace_first_pat_by_nth_pat(tcx, &mut new_body) {
            // An error has been emitted, so compilation fails. Do not transform the loops.
            return (false, new_body.into());
        }
        // The original loop positions, before new blocks are added to the body.
        let loop_positions = self.get_loop_positions(&new_body, tcx);
        self.loop_modifies_bindings =
            self.find_loop_modifies_bindings(&new_body, tcx, &loop_positions);
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
        self.add_generated_loop_modifies(&new_body, tcx, &loop_positions);
        (contain_loop_contracts, new_body.into())
    }

    /// Find the `kani_loop_modifies` binding of the loop modifies clause of each loop (see
    /// `loop_modifies` in library/kani_macros/src/sysroot/loop_contracts), and return it with the
    /// instance of the register function of the loop, so that codegen attaches the clause to that
    /// loop rather than to the next loop latch that it generates. The latch of an inner loop is
    /// generated before the latch of its outer loop, and the latch of a loop that never iterates
    /// (e.g. `loop { break; }`) is not generated at all.
    ///
    /// The binding of a loop is the last assignment of a `kani_loop_modifies` binding found by
    /// walking up the dominator tree from the loop head: the binding is assigned right before its
    /// loop, so it dominates the loop head, and the code that `#[kani::loop_invariant]` generates
    /// in between (e.g. the `on_entry` variables, which can branch, or the rewrite of a `for` loop)
    /// does not get in the way. The walk stops at the head of another loop and at a binding of
    /// another loop, and the loops are visited in reverse postorder, so a loop never takes the
    /// binding of a loop before it. A binding that is not found this way (e.g. that of a loop
    /// without `#[kani::loop_invariant]`) is not attached to any loop.
    fn find_loop_modifies_bindings(
        &self,
        body: &MutableBody,
        tcx: TyCtxt,
        loop_positions: &[(usize, usize)],
    ) -> Vec<(Local, Instance)> {
        let binding_locals: HashSet<usize> = body
            .var_debug_info()
            .iter()
            .filter(|info| info.name == "kani_loop_modifies")
            .filter_map(|info| info.local())
            .collect();
        if binding_locals.is_empty() {
            return Vec::new();
        }
        let (immediate_dominators, rpo_number) = Self::immediate_dominators(body);
        let mut loop_heads: Vec<usize> = loop_positions.iter().map(|(head, _)| *head).collect();
        loop_heads.sort_by_key(|head| rpo_number[*head]);
        let dominates = |dominator: usize, mut block: usize| loop {
            if block == dominator {
                return true;
            }
            match immediate_dominators[block] {
                Some(idom) => block = idom,
                None => return false,
            }
        };
        let mut predecessors = vec![Vec::new(); body.blocks().len()];
        for (block, data) in body.blocks().iter().enumerate() {
            for successor in data.terminator.successors() {
                predecessors[successor].push(block);
            }
        }
        // The head of a loop with or without a loop contract: the head of a loop with a loop
        // contract that does not iterate (e.g. `loop { break; }`) is not a back edge target.
        let is_any_loop_head = |block: usize| {
            self.is_loop_head(body, tcx, block)
                || predecessors[block].iter().any(|pred| dominates(block, *pred))
        };

        // The bindings found so far, by their position (block, statement index).
        let mut found: HashSet<(usize, usize)> = HashSet::new();
        let mut bindings = Vec::new();
        for loop_head in loop_heads {
            let Some((register_fn, _)) = self.register_fn(body, tcx, loop_head) else { continue };
            let mut block = Some(loop_head);
            'walk: while let Some(current) = block {
                for (stmt_idx, stmt) in body.blocks()[current].statements.iter().enumerate().rev() {
                    if let StatementKind::Assign(place, _) = &stmt.kind
                        && binding_locals.contains(&place.local)
                    {
                        // Either the binding of this loop, or a binding of another loop, after
                        // which the binding of this loop cannot be.
                        if found.insert((current, stmt_idx)) {
                            bindings.push((place.local, register_fn));
                        }
                        break 'walk;
                    }
                }
                // Stop at the head of an enclosing or preceding loop, before its own binding.
                block =
                    immediate_dominators[current].filter(|dominator| !is_any_loop_head(*dominator));
            }
        }
        bindings
    }

    /// The immediate dominator of each block that is reachable from the entry block (`None` for
    /// the entry block and the unreachable blocks), computed with the algorithm of Cooper, Harvey
    /// and Kennedy, "A Simple, Fast Dominance Algorithm", and the reverse postorder number of each
    /// block (`usize::MAX` for the unreachable blocks).
    fn immediate_dominators(body: &MutableBody) -> (Vec<Option<usize>>, Vec<usize>) {
        let num_blocks = body.blocks().len();
        // The postorder of the blocks that are reachable from the entry block.
        let mut postorder = Vec::with_capacity(num_blocks);
        let mut visited = vec![false; num_blocks];
        let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
        visited[0] = true;
        while let Some((block, next_successor)) = stack.pop() {
            let successors = body.blocks()[block].terminator.successors();
            if let Some(&successor) = successors.get(next_successor) {
                stack.push((block, next_successor + 1));
                if !visited[successor] {
                    visited[successor] = true;
                    stack.push((successor, 0));
                }
            } else {
                postorder.push(block);
            }
        }
        let mut rpo_number = vec![usize::MAX; num_blocks];
        for (number, block) in postorder.iter().rev().enumerate() {
            rpo_number[*block] = number;
        }
        let mut predecessors = vec![Vec::new(); num_blocks];
        for &block in &postorder {
            for successor in body.blocks()[block].terminator.successors() {
                predecessors[successor].push(block);
            }
        }
        let intersect = |idom: &[Option<usize>], mut a: usize, mut b: usize| {
            while a != b {
                while rpo_number[a] > rpo_number[b] {
                    a = idom[a].unwrap();
                }
                while rpo_number[b] > rpo_number[a] {
                    b = idom[b].unwrap();
                }
            }
            a
        };
        let mut idom: Vec<Option<usize>> = vec![None; num_blocks];
        idom[0] = Some(0);
        let mut changed = true;
        while changed {
            changed = false;
            // All blocks in reverse postorder, except for the entry block.
            for &block in postorder.iter().rev().skip(1) {
                let mut new_idom = None;
                for &pred in &predecessors[block] {
                    if idom[pred].is_some() {
                        new_idom = Some(match new_idom {
                            None => pred,
                            Some(other) => intersect(&idom, pred, other),
                        });
                    }
                }
                if new_idom != idom[block] {
                    idom[block] = new_idom;
                    changed = true;
                }
            }
        }
        idom[0] = None;
        (idom, rpo_number)
    }

    /// Record the locals in [LoopContractPass::generated_loop_locals], and the variables that
    /// `#[kani::loop_invariant]` generates for the loop, in
    /// [LoopContractPass::generated_loop_modifies], so that codegen adds them to the loop
    /// modifies clause of their loop if the user wrote one.
    ///
    /// Such a local is written both before the loop (by the `for` loop rewrite, or by the
    /// initialization that this pass copies to the loop head) and in the loop, so CBMC requires
    /// it to be in the loop's write set. The user cannot name it in a `#[kani::loop_modifies]`
    /// clause: it is the index, a pattern binding or a temporary of the `for` loop rewrite, a
    /// variable generated for `prev`, or a variable or temporary declared in the loop body.
    /// Allowing the loop to write to these locals does not allow it to write to any memory
    /// that is visible outside of the loop, which is still checked against the user's clause.
    ///
    /// The locals are keyed by the instance of the register function that the new loop latch
    /// calls, which is unique to the loop and is what the codegen hook of that call receives
    /// (see `LoopInvariantRegister` in codegen_cprover_gotoc/overrides/hooks.rs), and by the
    /// instance of the body (in case MIR inlining copies the loop into another body). This needs
    /// no change to the body, so later passes do not instrument anything for it, and it does not
    /// depend on the name of a local that MIR optimizations could remove.
    fn add_generated_loop_modifies(
        &mut self,
        body: &MutableBody,
        tcx: TyCtxt,
        loop_positions: &[(usize, usize)],
    ) {
        let mut new_latches: Vec<(usize, usize)> =
            self.new_loop_latches.iter().map(|(head, latch)| (*head, *latch)).collect();
        new_latches.sort();
        for (loop_head, new_latch) in new_latches {
            let Some((register_fn, loop_id)) = self.register_fn(body, tcx, new_latch) else {
                continue;
            };
            let Some((_, loop_latch)) = loop_positions.iter().find(|(head, _)| *head == loop_head)
            else {
                continue;
            };
            // The variables generated by `#[kani::loop_invariant]` for this loop that are
            // assigned before the loop and in each iteration: the index of a `for` loop, and the
            // variables for `prev`. Their names start with the loop id, which is unique to the
            // loop. These are the names of the variables, which, unlike the bindings of
            // `#[kani::loop_modifies]` or `#[kani::loop_decreases]`, are borrowed by the loop
            // invariant closure or assigned in the loop, so MIR optimizations keep them.
            let index_name = format!("kani_index{loop_id}");
            let prev_prefix = format!("__kani_prev_var{loop_id}_");
            let mut candidates: Vec<usize> = body
                .var_debug_info()
                .iter()
                .filter(|info| info.name == index_name || info.name.starts_with(&prev_prefix))
                .filter_map(|info| info.local())
                .collect();
            if let Some(locals) = self.generated_loop_locals.get(&loop_head) {
                candidates.extend(locals.iter().copied());
            }
            // Only keep the locals that the loop writes to, e.g., not a `kaniiter` whose
            // initialization was moved to the head of an outer loop.
            let written = Self::locals_written_in(body, loop_head + 1..=*loop_latch);
            let mut generated: Vec<usize> = candidates
                .into_iter()
                .filter(|local| written.contains(local))
                // Zero-sized locals cannot be modified, and CBMC does not support them as targets.
                .filter(|local| {
                    !body.locals()[*local]
                        .ty
                        .layout()
                        .is_ok_and(|layout| layout.shape().size.bytes() == 0)
                })
                .collect();
            generated.sort();
            generated.dedup();
            if !generated.is_empty() {
                self.generated_loop_modifies.push((register_fn, generated));
            }
        }
    }

    /// The instance of the register function called at the end of `block`, and the suffix
    /// `_<line>_<col>_<line>_<col>` that `#[kani::loop_invariant]` appends to the register
    /// function and to the other names it generates for the loop.
    fn register_fn(
        &self,
        body: &MutableBody,
        tcx: TyCtxt,
        block: usize,
    ) -> Option<(Instance, String)> {
        let TerminatorKind::Call { func, .. } = &body.blocks()[block].terminator.kind else {
            return None;
        };
        let RigidTy::FnDef(fn_def, args) = func.ty(body.locals()).ok()?.kind().rigid()?.clone()
        else {
            return None;
        };
        if KaniAttributes::for_def_id(tcx, fn_def.def_id()).fn_marker()
            != Some(Symbol::intern("kani_register_loop_contract"))
        {
            return None;
        }
        let name = fn_def.name();
        let loop_id = name.rsplit("::").next()?.strip_prefix("kani_register_loop_contract")?;
        Some((Instance::resolve(fn_def, &args).ok()?, loop_id.to_string()))
    }

    /// The locals that are assigned (as the root of the assigned place, or as the destination
    /// of a call) in the given blocks. A write through a pointer, e.g. `(*p).f = ..`, counts as
    /// a write to `p`, which is fine to filter the candidates of a loop.
    fn locals_written_in(
        body: &MutableBody,
        blocks: std::ops::RangeInclusive<usize>,
    ) -> HashSet<usize> {
        let mut written = HashSet::new();
        for block in blocks {
            let Some(block) = body.blocks().get(block) else { continue };
            for stmt in &block.statements {
                if let StatementKind::Assign(place, _) = &stmt.kind {
                    written.insert(place.local);
                }
            }
            if let TerminatorKind::Call { destination, .. } = &block.terminator.kind {
                written.insert(destination.local);
            }
        }
        written
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
                    source_info: synthetic_source_info(terminator.source_info.span),
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
                    source_info: synthetic_source_info(terminator.source_info.span),
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
                // We look for the args' types of the kani_register_loop_contract function
                // They are always stored in a tuple, which is next to the FnPtr generic args of kani_registered_loop_contract fn
                // All the generic args before the FnPtr are from the outer function
                let mut fnptrpos = 0;
                for (i, arg) in genarg.0.iter().enumerate() {
                    if let GenericArgKind::Type(arg_ty) = arg
                        && let TyKind::RigidTy(RigidTy::FnPtr(_)) = arg_ty.kind()
                    {
                        fnptrpos = i;
                        break;
                    }
                }
                let GenericArgKind::Type(arg_ty) = genarg.0[fnptrpos + 1] else { return false };
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
                            Rvalue::Ref(_,_,rplace) | Rvalue::CopyForDeref(rplace) | Rvalue::Use(Operand::Copy(rplace), _) => {
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
                    Rvalue::Use(
                        Operand::Constant(ConstOperand {
                            span: terminator.source_info.span,
                            user_ty: None,
                            const_: MirConst::from_bool(true),
                        }),
                        WithRetag::No,
                    ),
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
                        span: terminator.source_info.span,
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
                        source_info: synthetic_source_info(terminator.source_info.span),
                    },
                );
                new_body.replace_terminator(
                    &SourceInstruction::Terminator { bb: bb_idx },
                    Terminator {
                        kind: TerminatorKind::Goto { target: terminator_target.unwrap() },
                        source_info: synthetic_source_info(terminator.source_info.span),
                    },
                );
                // Cache the new loop latch.
                self.new_loop_latches.insert(bb_idx, new_body.blocks().len() - 1);
            }
        }
        contain_loop_contracts
    }
}
