// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of the points-to analysis using Rust's native dataflow framework. This provides
//! necessary aliasing information for instrumenting delayed UB later on.
//!
//! The analysis uses Rust's dataflow framework by implementing appropriate traits to leverage the
//! existing fixpoint solver infrastructure. The main trait responsible for the dataflow analysis
//! behavior is `rustc_mir_dataflow::Analysis`: it provides two methods that are responsible for
//! handling statements and terminators, which we implement.
//!
//! The analysis proceeds by looking at each instruction in the dataflow order and collecting all
//! possible aliasing relations that the instruction introduces. If a terminator is a function call,
//! the analysis recurs into the function and then joins the information retrieved from it into the
//! original graph.
//!
//! For each instruction, the analysis first resolves dereference projections for each place to
//! determine which places it could point to. This is done by finding a set of successors in the
//! graph for each dereference projection.
//!
//! Then, the analysis adds the appropriate edges into the points-to graph. It proceeds until there
//! is no new information to be discovered.
//!
//! Currently, the analysis is not field-sensitive: e.g., if a field of a place aliases to some
//! other place, we treat it as if the place itself aliases to another place.

use crate::{
    intrinsics::Intrinsic,
    kani_middle::{
        points_to::{MemLoc, PointsToGraph},
        reachability::CallGraph,
        transform::RustcInternalMir,
    },
};
use rustc_middle::{
    mir::{
        BasicBlock, BinOp, Body, CallReturnPlaces, Location, NonDivergingIntrinsic, Operand, Place,
        ProjectionElem, Rvalue, Statement, StatementKind, Terminator, TerminatorEdges,
        TerminatorKind,
    },
    ty::{Instance, InstanceKind, List, ParamEnv, TyCtxt, TyKind},
};
use rustc_mir_dataflow::{Analysis, AnalysisDomain, Forward, JoinSemiLattice};
use rustc_smir::rustc_internal;
use rustc_span::{source_map::Spanned, DUMMY_SP};
use stable_mir::mir::{mono::Instance as StableInstance, Body as StableBody};
use std::collections::HashSet;

/// Main points-to analysis object.
struct PointsToAnalysis<'a, 'tcx> {
    instance: Instance<'tcx>,
    body: &'a Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    /// This will be used in the future to resolve function pointer and vtable calls. Currently, we
    /// can resolve call graph edges just by looking at the terminators and erroring if we can't
    /// resolve the callee.  
    call_graph: &'a CallGraph,
    /// This graph should contain a subset of the points-to graph reachable from function arguments.
    /// For the entry function it will be empty (as it supposedly does not have any parameters).
    initial_graph: PointsToGraph<'tcx>,
}

/// Public points-to analysis entry point. Performs the analysis on a body, outputting the graph
/// containing aliasing information of the body itself and any body reachable from it.
pub fn run_points_to_analysis<'tcx>(
    body: &StableBody,
    tcx: TyCtxt<'tcx>,
    instance: StableInstance,
    call_graph: &CallGraph,
) -> PointsToGraph<'tcx> {
    // Dataflow analysis does not yet work with StableMIR, so need to perform backward
    // conversion.
    let internal_instance = rustc_internal::internal(tcx, instance);
    let internal_body = body.internal_mir(tcx);
    PointsToAnalysis::run(
        &internal_body,
        tcx,
        internal_instance,
        call_graph,
        PointsToGraph::empty(),
    )
}

impl<'a, 'tcx> PointsToAnalysis<'a, 'tcx> {
    /// Perform the analysis on a body, outputting the graph containing aliasing information of the
    /// body itself and any body reachable from it.
    pub fn run(
        body: &'a Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        instance: Instance<'tcx>,
        call_graph: &'a CallGraph,
        initial_graph: PointsToGraph<'tcx>,
    ) -> PointsToGraph<'tcx> {
        let analysis = Self { body, tcx, instance, call_graph, initial_graph };
        // This creates a fixpoint solver using the initial graph, the body, and extra information
        // and solves the dataflow problem, producing the cursor, which contains dataflow state for
        // each instruction in the body.
        let mut cursor =
            analysis.into_engine(tcx, body).iterate_to_fixpoint().into_results_cursor(body);
        // We collect dataflow state at each `Return` terminator to determine the full aliasing
        // graph for the function. This is sound since those are the only places where the function
        // finishes, so the dataflow state at those places will be a union of dataflow states
        // preceding to it, which means every possible execution is taken into account.
        let mut results = PointsToGraph::empty();
        for (idx, bb) in body.basic_blocks.iter().enumerate() {
            if let TerminatorKind::Return = bb.terminator().kind {
                // Switch the cursor to the end of the block ending with `Return`.
                cursor.seek_to_block_end(idx.into());
                // Retrieve the dataflow state and join into the results graph.
                results.join(&cursor.get().clone());
            }
        }
        results
    }
}

impl<'a, 'tcx> AnalysisDomain<'tcx> for PointsToAnalysis<'a, 'tcx> {
    /// Dataflow state at each instruction.
    type Domain = PointsToGraph<'tcx>;

    type Direction = Forward;

    const NAME: &'static str = "PointsToAnalysis";

    /// Dataflow state instantiated at the beginning of each basic block, before the state from
    /// previous basic blocks gets joined into it.
    fn bottom_value(&self, _body: &Body<'tcx>) -> Self::Domain {
        PointsToGraph::empty()
    }

    /// Dataflow state instantiated at the entry into the body; this should be the initial dataflow
    /// graph.
    fn initialize_start_block(&self, _body: &Body<'tcx>, state: &mut Self::Domain) {
        state.join(&self.initial_graph.clone());
    }
}

impl<'a, 'tcx> Analysis<'tcx> for PointsToAnalysis<'a, 'tcx> {
    /// Update current dataflow state based on the information we can infer from the given
    /// statement.
    fn apply_statement_effect(
        &mut self,
        state: &mut Self::Domain,
        statement: &Statement<'tcx>,
        _location: Location,
    ) {
        // The only two statements that can introduce new aliasing information are assignments and
        // copies using `copy_nonoverlapping`.
        match &statement.kind {
            StatementKind::Assign(assign_box) => {
                let (place, rvalue) = *assign_box.clone();
                // Resolve all dereference projections for the lvalue.
                let lvalue_set = state.resolve_place(place, self.instance);
                // Determine all places rvalue could point to.
                let rvalue_set = self.successors_for_rvalue(state, rvalue);
                // Create an edge between all places which could be lvalue and all places rvalue
                // could be pointing to.
                state.extend(&lvalue_set, &rvalue_set);
            }
            StatementKind::Intrinsic(non_diverging_intrinsic) => {
                match *non_diverging_intrinsic.clone() {
                    NonDivergingIntrinsic::CopyNonOverlapping(copy_nonoverlapping) => {
                        // Copy between the values pointed by `*const a` and `*mut b` is
                        // semantically equivalent to *b = *a with respect to aliasing.
                        self.apply_copy_effect(
                            state,
                            copy_nonoverlapping.src.clone(),
                            copy_nonoverlapping.dst.clone(),
                        );
                    }
                    NonDivergingIntrinsic::Assume(..) => { /* This is a no-op. */ }
                }
            }
            StatementKind::FakeRead(..)
            | StatementKind::SetDiscriminant { .. }
            | StatementKind::Deinit(..)
            | StatementKind::StorageLive(..)
            | StatementKind::StorageDead(..)
            | StatementKind::Retag(..)
            | StatementKind::PlaceMention(..)
            | StatementKind::AscribeUserType(..)
            | StatementKind::Coverage(..)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop => { /* This is a no-op with regard to aliasing. */ }
        }
    }

    fn apply_terminator_effect<'mir>(
        &mut self,
        state: &mut Self::Domain,
        terminator: &'mir Terminator<'tcx>,
        location: Location,
    ) -> TerminatorEdges<'mir, 'tcx> {
        if let TerminatorKind::Call { func, args, destination, .. } = &terminator.kind {
            // Attempt to resolve callee. For now, we panic if the callee cannot be resolved (e.g.,
            // if a function pointer call is used), but we could leverage the call graph to resolve
            // it.
            let instance = match try_resolve_instance(self.body, func, self.tcx) {
                Ok(instance) => instance,
                Err(reason) => {
                    unimplemented!("{reason}")
                }
            };
            match instance.def {
                // Intrinsics could introduce aliasing edges we care about, so need to handle them.
                InstanceKind::Intrinsic(_) => {
                    match Intrinsic::from_instance(&rustc_internal::stable(instance)) {
                        intrinsic if is_identity_aliasing_intrinsic(intrinsic.clone()) => {
                            // Treat the intrinsic as an aggregate, taking a union of all of the
                            // arguments' aliases.
                            let destination_set = state.resolve_place(*destination, self.instance);
                            let operands_set = args
                                .into_iter()
                                .flat_map(|operand| {
                                    self.successors_for_operand(state, operand.node.clone())
                                })
                                .collect();
                            state.extend(&destination_set, &operands_set);
                        }
                        // All `atomic_cxchg` intrinsics take `dst, old, src` as arguments.
                        // This is equivalent to `destination = *dst; *dst = src`.
                        Intrinsic::AtomicCxchg(_) | Intrinsic::AtomicCxchgWeak(_) => {
                            let src_set = self.successors_for_operand(state, args[2].node.clone());
                            let dst_set = self.successors_for_deref(state, args[0].node.clone());
                            let destination_set = state.resolve_place(*destination, self.instance);
                            state.extend(&destination_set, &state.successors(&dst_set));
                            state.extend(&dst_set, &src_set);
                        }
                        // All `atomic_load` intrinsics take `src` as an argument.
                        // This is equivalent to `destination = *src`.
                        Intrinsic::AtomicLoad(_) => {
                            let src_set = self.successors_for_deref(state, args[0].node.clone());
                            let destination_set = state.resolve_place(*destination, self.instance);
                            state.extend(&destination_set, &state.successors(&src_set));
                        }
                        // All `atomic_store` intrinsics take `dst, val` as arguments.
                        // This is equivalent to `*dst = val`.
                        Intrinsic::AtomicStore(_) => {
                            let dst_set = self.successors_for_deref(state, args[0].node.clone());
                            let val_set = self.successors_for_operand(state, args[1].node.clone());
                            state.extend(&dst_set, &val_set);
                        }
                        // All other `atomic` intrinsics take `dst, src` as arguments.
                        // This is equivalent to `destination = *dst; *dst = src`.
                        Intrinsic::AtomicAnd(_)
                        | Intrinsic::AtomicMax(_)
                        | Intrinsic::AtomicMin(_)
                        | Intrinsic::AtomicNand(_)
                        | Intrinsic::AtomicOr(_)
                        | Intrinsic::AtomicUmax(_)
                        | Intrinsic::AtomicUmin(_)
                        | Intrinsic::AtomicXadd(_)
                        | Intrinsic::AtomicXchg(_)
                        | Intrinsic::AtomicXor(_)
                        | Intrinsic::AtomicXsub(_) => {
                            let src_set = self.successors_for_operand(state, args[1].node.clone());
                            let dst_set = self.successors_for_deref(state, args[0].node.clone());
                            let destination_set = state.resolve_place(*destination, self.instance);
                            state.extend(&destination_set, &state.successors(&dst_set));
                            state.extend(&dst_set, &src_set);
                        }
                        // Similar to `copy_nonoverlapping`, argument order is `src`, `dst`, `count`.
                        Intrinsic::Copy => {
                            self.apply_copy_effect(
                                state,
                                args[0].node.clone(),
                                args[1].node.clone(),
                            );
                        }
                        Intrinsic::TypedSwap => {
                            // Extend from x_set to y_set and vice-versa so that both x and y alias
                            // to a union of places each of them alias to.
                            let x_set = self.successors_for_deref(state, args[0].node.clone());
                            let y_set = self.successors_for_deref(state, args[1].node.clone());
                            state.extend(&x_set, &state.successors(&y_set));
                            state.extend(&y_set, &state.successors(&x_set));
                        }
                        // Similar to `copy_nonoverlapping`, argument order is `dst`, `src`, `count`.
                        Intrinsic::VolatileCopyMemory
                        | Intrinsic::VolatileCopyNonOverlappingMemory => {
                            self.apply_copy_effect(
                                state,
                                args[1].node.clone(),
                                args[0].node.clone(),
                            );
                        }
                        // Semantically equivalent to dest = *a
                        Intrinsic::VolatileLoad | Intrinsic::UnalignedVolatileLoad => {
                            // Destination of the return value.
                            let lvalue_set = state.resolve_place(*destination, self.instance);
                            let rvalue_set = self.successors_for_deref(state, args[0].node.clone());
                            state.extend(&lvalue_set, &state.successors(&rvalue_set));
                        }
                        // Semantically equivalent *a = b.
                        Intrinsic::VolatileStore => {
                            let lvalue_set = self.successors_for_deref(state, args[0].node.clone());
                            let rvalue_set =
                                self.successors_for_operand(state, args[1].node.clone());
                            state.extend(&lvalue_set, &rvalue_set);
                        }
                        Intrinsic::Unimplemented { .. } => {
                            // This will be taken care of at the codegen level.
                        }
                        intrinsic => {
                            unimplemented!(
                                "Kani does not support reasoning about aliasing in presence of intrinsic `{intrinsic:?}`. For more information about the state of uninitialized memory checks implementation, see: https://github.com/model-checking/kani/issues/3300."
                            );
                        }
                    }
                }
                _ => {
                    if self.tcx.is_foreign_item(instance.def_id()) {
                        match self
                            .tcx
                            .def_path_str_with_args(instance.def_id(), instance.args)
                            .as_str()
                        {
                            // This is an internal function responsible for heap allocation,
                            // which creates a new node we need to add to the points-to graph.
                            "alloc::alloc::__rust_alloc" | "alloc::alloc::__rust_alloc_zeroed" => {
                                let lvalue_set = state.resolve_place(*destination, self.instance);
                                let rvalue_set = HashSet::from([MemLoc::new_heap_allocation(
                                    self.instance,
                                    location,
                                )]);
                                state.extend(&lvalue_set, &rvalue_set);
                            }
                            _ => {}
                        }
                    } else {
                        // Otherwise, handle this as a regular function call.
                        self.apply_regular_call_effect(state, instance, args, destination);
                    }
                }
            }
        };
        terminator.edges()
    }

    /// We don't care about this and just need to implement this to implement the trait.
    fn apply_call_return_effect(
        &mut self,
        _state: &mut Self::Domain,
        _block: BasicBlock,
        _return_places: CallReturnPlaces<'_, 'tcx>,
    ) {
    }
}

/// Try retrieving instance for the given function operand.
fn try_resolve_instance<'tcx>(
    body: &Body<'tcx>,
    func: &Operand<'tcx>,
    tcx: TyCtxt<'tcx>,
) -> Result<Instance<'tcx>, String> {
    let ty = func.ty(body, tcx);
    match ty.kind() {
        TyKind::FnDef(def, args) => {
            // Span here is used for error-reporting, which we don't expect to encounter anyway, so
            // it is ok to use a dummy.
            Ok(Instance::expect_resolve(tcx, ParamEnv::reveal_all(), *def, &args, DUMMY_SP))
        }
        _ => Err(format!(
            "Kani was not able to resolve the instance of the function operand `{ty:?}`. Currently, memory initialization checks in presence of function pointers and vtable calls are not supported. For more information about planned support, see https://github.com/model-checking/kani/issues/3300."
        )),
    }
}

impl<'a, 'tcx> PointsToAnalysis<'a, 'tcx> {
    /// Update the analysis state according to the operation, which is semantically equivalent to `*to = *from`.
    fn apply_copy_effect(
        &self,
        state: &mut PointsToGraph<'tcx>,
        from: Operand<'tcx>,
        to: Operand<'tcx>,
    ) {
        let lvalue_set = self.successors_for_deref(state, to);
        let rvalue_set = self.successors_for_deref(state, from);
        state.extend(&lvalue_set, &state.successors(&rvalue_set));
    }

    /// Find all places where the operand could point to at the current stage of the program.
    fn successors_for_operand(
        &self,
        state: &mut PointsToGraph<'tcx>,
        operand: Operand<'tcx>,
    ) -> HashSet<MemLoc<'tcx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                // Find all places which are pointed to by the place.
                state.successors(&state.resolve_place(place, self.instance))
            }
            Operand::Constant(const_operand) => {
                // Constants could point to a static, so need to check for that.
                if let Some(static_def_id) = const_operand.check_static_ptr(self.tcx) {
                    HashSet::from([MemLoc::new_static_allocation(static_def_id)])
                } else {
                    HashSet::new()
                }
            }
        }
    }

    /// Find all places where the deref of the operand could point to at the current stage of the program.
    fn successors_for_deref(
        &self,
        state: &mut PointsToGraph<'tcx>,
        operand: Operand<'tcx>,
    ) -> HashSet<MemLoc<'tcx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => state.resolve_place(
                place.project_deeper(&[ProjectionElem::Deref], self.tcx),
                self.instance,
            ),
            Operand::Constant(const_operand) => {
                // Constants could point to a static, so need to check for that.
                if let Some(static_def_id) = const_operand.check_static_ptr(self.tcx) {
                    HashSet::from([MemLoc::new_static_allocation(static_def_id)])
                } else {
                    HashSet::new()
                }
            }
        }
    }

    /// Update the analysis state according to the regular function call.
    fn apply_regular_call_effect(
        &mut self,
        state: &mut PointsToGraph<'tcx>,
        instance: Instance<'tcx>,
        args: &[Spanned<Operand<'tcx>>],
        destination: &Place<'tcx>,
    ) {
        // Here we simply call another function, so need to retrieve internal body for it.
        let new_body = {
            let stable_instance = rustc_internal::stable(instance);
            let stable_body = stable_instance.body().unwrap();
            stable_body.internal_mir(self.tcx)
        };

        // In order to be efficient, create a new graph for the function call analysis, which only
        // contains arguments and statics and anything transitively reachable from them.
        let mut initial_graph = PointsToGraph::empty();
        for arg in args.iter() {
            match arg.node {
                Operand::Copy(place) | Operand::Move(place) => {
                    initial_graph
                        .join(&state.transitive_closure(state.resolve_place(place, self.instance)));
                }
                Operand::Constant(_) => {}
            }
        }

        // A missing link is the connections between the arguments in the caller and parameters in
        // the callee, add it to the graph.
        if self.tcx.is_closure_like(instance.def.def_id()) {
            // This means we encountered a closure call.
            // Sanity check. The first argument is the closure itself and the second argument is the tupled arguments from the caller.
            assert!(args.len() == 2);
            // First, connect all upvars.
            let lvalue_set = HashSet::from([MemLoc::new_stack_allocation(
                instance,
                Place { local: 1usize.into(), projection: List::empty() },
            )]);
            let rvalue_set = self.successors_for_operand(state, args[0].node.clone());
            initial_graph.extend(&lvalue_set, &rvalue_set);
            // Then, connect the argument tuple to each of the spread arguments.
            let spread_arg_operand = args[1].node.clone();
            for i in 0..new_body.arg_count {
                let lvalue_set = HashSet::from([MemLoc::new_stack_allocation(
                    instance,
                    Place {
                        local: (i + 1).into(), // Since arguments in the callee are starting with 1, account for that.
                        projection: List::empty(),
                    },
                )]);
                // This conservatively assumes all arguments alias to all parameters.
                let rvalue_set = self.successors_for_operand(state, spread_arg_operand.clone());
                initial_graph.extend(&lvalue_set, &rvalue_set);
            }
        } else {
            // Otherwise, simply connect all arguments to parameters.
            for (i, arg) in args.iter().enumerate() {
                let lvalue_set = HashSet::from([MemLoc::new_stack_allocation(
                    instance,
                    Place {
                        local: (i + 1).into(), // Since arguments in the callee are starting with 1, account for that.
                        projection: List::empty(),
                    },
                )]);
                let rvalue_set = self.successors_for_operand(state, arg.node.clone());
                initial_graph.extend(&lvalue_set, &rvalue_set);
            }
        }

        // Run the analysis.
        let new_result =
            PointsToAnalysis::run(&new_body, self.tcx, instance, self.call_graph, initial_graph);
        // Merge the results into the current state.
        state.join(&new_result);

        // Connect the return value to the return destination.
        let lvalue_set = state.resolve_place(*destination, self.instance);
        let rvalue_set = HashSet::from([MemLoc::new_stack_allocation(
            instance,
            Place { local: 0usize.into(), projection: List::empty() },
        )]);
        state.extend(&lvalue_set, &state.successors(&rvalue_set));
    }

    /// Find all places where the rvalue could point to at the current stage of the program.
    fn successors_for_rvalue(
        &self,
        state: &mut PointsToGraph<'tcx>,
        rvalue: Rvalue<'tcx>,
    ) -> HashSet<MemLoc<'tcx>> {
        match rvalue {
            // Using the operand unchanged requires determining where it could point, which
            // `successors_for_operand` does.
            Rvalue::Use(operand)
            | Rvalue::ShallowInitBox(operand, _)
            | Rvalue::Cast(_, operand, _)
            | Rvalue::Repeat(operand, ..) => self.successors_for_operand(state, operand),
            Rvalue::Ref(_, _, ref_place) | Rvalue::AddressOf(_, ref_place) => {
                // Here, a reference to a place is created, which leaves the place
                // unchanged.
                state.resolve_place(ref_place, self.instance)
            }
            Rvalue::BinaryOp(bin_op, operands) => {
                match bin_op {
                    BinOp::Offset => {
                        // Offsetting a pointer should still be within the boundaries of the
                        // same object, so we can simply use the operand unchanged.
                        let (ptr, _) = *operands.clone();
                        self.successors_for_operand(state, ptr)
                    }
                    BinOp::Add
                    | BinOp::AddUnchecked
                    | BinOp::AddWithOverflow
                    | BinOp::Sub
                    | BinOp::SubUnchecked
                    | BinOp::SubWithOverflow
                    | BinOp::Mul
                    | BinOp::MulUnchecked
                    | BinOp::MulWithOverflow
                    | BinOp::Div
                    | BinOp::Rem
                    | BinOp::BitXor
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::Shl
                    | BinOp::ShlUnchecked
                    | BinOp::Shr
                    | BinOp::ShrUnchecked => {
                        // While unlikely, those could be pointer addresses, so we need to
                        // track them. We assume that even shifted addresses will be within
                        // the same original object.
                        let (l_operand, r_operand) = *operands.clone();
                        let l_operand_set = self.successors_for_operand(state, l_operand);
                        let r_operand_set = self.successors_for_operand(state, r_operand);
                        l_operand_set.union(&r_operand_set).cloned().collect()
                    }
                    BinOp::Eq
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Ne
                    | BinOp::Ge
                    | BinOp::Gt
                    | BinOp::Cmp => {
                        // None of those could yield an address as the result.
                        HashSet::new()
                    }
                }
            }
            Rvalue::UnaryOp(_, operand) => {
                // The same story from BinOp applies here, too. Need to track those things.
                self.successors_for_operand(state, operand)
            }
            Rvalue::Len(..) | Rvalue::NullaryOp(..) | Rvalue::Discriminant(..) => {
                // All of those should yield a constant.
                HashSet::new()
            }
            Rvalue::Aggregate(_, operands) => {
                // Conservatively find a union of all places mentioned here and resolve
                // their pointees.
                operands
                    .into_iter()
                    .flat_map(|operand| self.successors_for_operand(state, operand))
                    .collect()
            }
            Rvalue::CopyForDeref(place) => {
                // Resolve pointees of a place.
                state.successors(&state.resolve_place(place, self.instance))
            }
            Rvalue::ThreadLocalRef(def_id) => {
                // We store a def_id of a static.
                HashSet::from([MemLoc::new_static_allocation(def_id)])
            }
        }
    }
}

/// Determines if the intrinsic does not influence aliasing beyond being treated as an identity
/// function (i.e. propagate aliasing without changes).
fn is_identity_aliasing_intrinsic(intrinsic: Intrinsic) -> bool {
    match intrinsic {
        Intrinsic::AddWithOverflow
        | Intrinsic::ArithOffset
        | Intrinsic::AssertInhabited
        | Intrinsic::AssertMemUninitializedValid
        | Intrinsic::AssertZeroValid
        | Intrinsic::Assume
        | Intrinsic::Bitreverse
        | Intrinsic::BlackBox
        | Intrinsic::Breakpoint
        | Intrinsic::Bswap
        | Intrinsic::CeilF32
        | Intrinsic::CeilF64
        | Intrinsic::CompareBytes
        | Intrinsic::CopySignF32
        | Intrinsic::CopySignF64
        | Intrinsic::CosF32
        | Intrinsic::CosF64
        | Intrinsic::Ctlz
        | Intrinsic::CtlzNonZero
        | Intrinsic::Ctpop
        | Intrinsic::Cttz
        | Intrinsic::CttzNonZero
        | Intrinsic::DiscriminantValue
        | Intrinsic::ExactDiv
        | Intrinsic::Exp2F32
        | Intrinsic::Exp2F64
        | Intrinsic::ExpF32
        | Intrinsic::ExpF64
        | Intrinsic::FabsF32
        | Intrinsic::FabsF64
        | Intrinsic::FaddFast
        | Intrinsic::FdivFast
        | Intrinsic::FloorF32
        | Intrinsic::FloorF64
        | Intrinsic::FmafF32
        | Intrinsic::FmafF64
        | Intrinsic::FmulFast
        | Intrinsic::Forget
        | Intrinsic::FsubFast
        | Intrinsic::IsValStaticallyKnown
        | Intrinsic::Likely
        | Intrinsic::Log10F32
        | Intrinsic::Log10F64
        | Intrinsic::Log2F32
        | Intrinsic::Log2F64
        | Intrinsic::LogF32
        | Intrinsic::LogF64
        | Intrinsic::MaxNumF32
        | Intrinsic::MaxNumF64
        | Intrinsic::MinAlignOf
        | Intrinsic::MinAlignOfVal
        | Intrinsic::MinNumF32
        | Intrinsic::MinNumF64
        | Intrinsic::MulWithOverflow
        | Intrinsic::NearbyIntF32
        | Intrinsic::NearbyIntF64
        | Intrinsic::NeedsDrop
        | Intrinsic::PowF32
        | Intrinsic::PowF64
        | Intrinsic::PowIF32
        | Intrinsic::PowIF64
        | Intrinsic::PrefAlignOf
        | Intrinsic::PtrGuaranteedCmp
        | Intrinsic::PtrOffsetFrom
        | Intrinsic::PtrOffsetFromUnsigned
        | Intrinsic::RawEq
        | Intrinsic::RetagBoxToRaw
        | Intrinsic::RintF32
        | Intrinsic::RintF64
        | Intrinsic::RotateLeft
        | Intrinsic::RotateRight
        | Intrinsic::RoundF32
        | Intrinsic::RoundF64
        | Intrinsic::SaturatingAdd
        | Intrinsic::SaturatingSub
        | Intrinsic::SinF32
        | Intrinsic::SinF64
        | Intrinsic::SizeOfVal
        | Intrinsic::SqrtF32
        | Intrinsic::SqrtF64
        | Intrinsic::SubWithOverflow
        | Intrinsic::Transmute
        | Intrinsic::TruncF32
        | Intrinsic::TruncF64
        | Intrinsic::TypeId
        | Intrinsic::TypeName
        | Intrinsic::UncheckedDiv
        | Intrinsic::UncheckedRem
        | Intrinsic::Unlikely
        | Intrinsic::VtableSize
        | Intrinsic::VtableAlign
        | Intrinsic::WrappingAdd
        | Intrinsic::WrappingMul
        | Intrinsic::WrappingSub
        | Intrinsic::WriteBytes => {
            /* Intrinsics that do not interact with aliasing beyond propagating it. */
            true
        }
        Intrinsic::SimdAdd
        | Intrinsic::SimdAnd
        | Intrinsic::SimdDiv
        | Intrinsic::SimdRem
        | Intrinsic::SimdEq
        | Intrinsic::SimdExtract
        | Intrinsic::SimdGe
        | Intrinsic::SimdGt
        | Intrinsic::SimdInsert
        | Intrinsic::SimdLe
        | Intrinsic::SimdLt
        | Intrinsic::SimdMul
        | Intrinsic::SimdNe
        | Intrinsic::SimdOr
        | Intrinsic::SimdShl
        | Intrinsic::SimdShr
        | Intrinsic::SimdShuffle(_)
        | Intrinsic::SimdSub
        | Intrinsic::SimdXor => {
            /* SIMD operations */
            true
        }
        Intrinsic::AtomicFence(_) | Intrinsic::AtomicSingleThreadFence(_) => {
            /* Atomic fences */
            true
        }
        _ => {
            /* Everything else */
            false
        }
    }
}
