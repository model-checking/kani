// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of points-to analysis using Rust's native dataflow framework. This provides
//! necessary aliasing information for instrumenting delayed UB later on.

use crate::kani_middle::{
    reachability::CallGraph,
    transform::{
        check_uninit::delayed_ub::points_to_graph::{GlobalMemLoc, LocalMemLoc, PointsToGraph},
        internal_mir::RustcInternalMir,
        BodyTransformation,
    },
};
use rustc_ast::Mutability;
use rustc_hir::def_id::DefId;
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
use rustc_span::source_map::Spanned;
use stable_mir::mir::mono::Instance as StableInstance;
use std::collections::HashSet;

/// Main points-to analysis object. Since this one will be created anew for each instance analysis,
/// we need to make sure big data structures are not copied unnecessarily.
pub struct PointsToAnalysis<'a, 'tcx> {
    def_id: DefId,
    body: Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    call_graph: &'a CallGraph,
    instances: &'a Vec<StableInstance>,
    transformer: &'a mut BodyTransformation,
}

impl<'a, 'tcx> PointsToAnalysis<'a, 'tcx> {
    /// Perform the analysis on a body, outputting the graph containing aliasing information of the
    /// body itself and any body reachable from it.
    pub fn run(
        body: Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        def_id: DefId,
        call_graph: &'a CallGraph,
        instances: &'a Vec<StableInstance>,
        transformer: &'a mut BodyTransformation,
    ) -> PointsToGraph<'tcx> {
        let analysis = Self { body: body.clone(), tcx, def_id, call_graph, instances, transformer };
        let mut cursor =
            analysis.into_engine(tcx, &body).iterate_to_fixpoint().into_results_cursor(&body);
        let mut results = PointsToGraph::from_body(&body, def_id);
        for (idx, _) in body.basic_blocks.iter().enumerate() {
            cursor.seek_to_block_end(idx.into());
            results.join(cursor.get());
        }
        results
    }
}

impl<'a, 'tcx> AnalysisDomain<'tcx> for PointsToAnalysis<'a, 'tcx> {
    type Domain = PointsToGraph<'tcx>;

    type Direction = Forward;

    const NAME: &'static str = "PointsToAnalysis";

    /// Dataflow state instantiated at the beginning of each basic block.
    fn bottom_value(&self, body: &Body<'tcx>) -> Self::Domain {
        PointsToGraph::from_body(body, self.def_id)
    }

    /// Dataflow state instantiated at the entry into the body, for us this coincides with the
    /// bottom value, so we don't need to do anything.
    fn initialize_start_block(
        &self,
        _body: &rustc_middle::mir::Body<'tcx>,
        _state: &mut Self::Domain,
    ) {
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
                // Lvalue is `place`, which is already available to us.
                let lvalue_set = state.follow_from_place(place, self.def_id);
                // Determine all places which the newly created rvalue could point to.
                let rvalue_set = match rvalue {
                    // Using the operand unchanged.
                    Rvalue::Use(operand)
                    | Rvalue::ShallowInitBox(operand, _)
                    | Rvalue::Cast(_, operand, _)
                    | Rvalue::Repeat(operand, ..) => self.follow_rvalue(state, operand),
                    Rvalue::Ref(_, _, place) | Rvalue::AddressOf(_, place) => {
                        // Here, a reference to a place is created, which leaves the place
                        // unchanged.
                        state.follow_from_place(place, self.def_id)
                    }
                    Rvalue::BinaryOp(bin_op, operands) => {
                        match bin_op {
                            BinOp::Offset => {
                                // Offsetting a pointer should still be within the boundaries of the
                                // same object, so we can simply use the operand unchanged.
                                let (ptr, _) = *operands.clone();
                                self.follow_rvalue(state, ptr)
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
                                let l_operand_set = self.follow_rvalue(state, l_operand);
                                let r_operand_set = self.follow_rvalue(state, r_operand);
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
                        self.follow_rvalue(state, operand)
                    }
                    Rvalue::Len(..) | Rvalue::NullaryOp(..) | Rvalue::Discriminant(..) => {
                        // All of those should yield a constant.
                        HashSet::new()
                    }
                    Rvalue::Aggregate(_, operands) => {
                        // Conservatively find a union of all places mentioned here.
                        operands
                            .into_iter()
                            .flat_map(|operand| self.follow_rvalue(state, operand))
                            .collect()
                    }
                    Rvalue::CopyForDeref(place) => {
                        // Use a place unchanged.
                        state.follow(&state.follow_from_place(place, self.def_id))
                    }
                    Rvalue::ThreadLocalRef(def_id) => HashSet::from([GlobalMemLoc::Global(def_id)]),
                };
                // Create an edge between all places which could be lvalue and all places rvalue
                // could be pointing to.
                state.extend(&lvalue_set, &rvalue_set);
            }
            StatementKind::Intrinsic(non_diverging_intrinsic) => {
                match *non_diverging_intrinsic.clone() {
                    NonDivergingIntrinsic::CopyNonOverlapping(copy_nonoverlapping) => {
                        // Copy between `*const a` and `*mut b` is semantically equivalent to *b =
                        // *a with respect to aliasing.
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
        _location: Location,
    ) -> TerminatorEdges<'mir, 'tcx> {
        if let TerminatorKind::Call { func, args, destination, .. } = &terminator.kind {
            let instance = match try_resolve_instance(&self.body, func, self.tcx) {
                Ok(instance) => instance,
                Err(reason) => {
                    unimplemented!("{reason}")
                }
            };
            match instance.def {
                // Intrinsics could introduce aliasing edges we care about.
                InstanceKind::Intrinsic(def_id) => {
                    match self.tcx.intrinsic(def_id).unwrap().name.to_string().as_str() {
                        name if name.starts_with("atomic") => {
                            match name {
                                // All `atomic_cxchg` intrinsics take `dst, old, src` as arguments.
                                // This is equivalent to `destination = *dst; *dst = src`.
                                name if name.starts_with("atomic_cxchg") => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `{name}`"
                                    );
                                    assert!(matches!(
                                        args[0].node.ty(&self.body, self.tcx).kind(),
                                        TyKind::RawPtr(_, Mutability::Mut)
                                    ));
                                    let src_set = self.follow_rvalue(state, args[2].node.clone());
                                    let dst_set = self.follow_deref(state, args[0].node.clone());
                                    let destination_set =
                                        state.follow_from_place(*destination, self.def_id);
                                    state.extend(&destination_set, &state.follow(&dst_set));
                                    state.extend(&dst_set, &src_set);
                                }
                                // All `atomic_load` intrinsics take `src` as an argument.
                                // This is equivalent to `destination = *src`.
                                name if name.starts_with("atomic_load") => {
                                    assert_eq!(
                                        args.len(),
                                        1,
                                        "Unexpected number of arguments for `{name}`"
                                    );
                                    assert!(matches!(
                                        args[0].node.ty(&self.body, self.tcx).kind(),
                                        TyKind::RawPtr(_, Mutability::Not)
                                    ));
                                    let src_set = self.follow_deref(state, args[0].node.clone());
                                    let destination_set =
                                        state.follow_from_place(*destination, self.def_id);
                                    state.extend(&destination_set, &state.follow(&src_set));
                                }
                                // All `atomic_store` intrinsics take `dst, val` as arguments.
                                // This is equivalent to `*dst = val`.
                                name if name.starts_with("atomic_store") => {
                                    assert_eq!(
                                        args.len(),
                                        2,
                                        "Unexpected number of arguments for `{name}`"
                                    );
                                    assert!(matches!(
                                        args[0].node.ty(&self.body, self.tcx).kind(),
                                        TyKind::RawPtr(_, Mutability::Mut)
                                    ));
                                    let dst_set = self.follow_deref(state, args[0].node.clone());
                                    let val_set = self.follow_rvalue(state, args[1].node.clone());
                                    state.extend(&dst_set, &val_set);
                                }
                                // All other `atomic` intrinsics take `dst, src` as arguments.
                                // This is equivalent to `destination = *dst; *dst = src`.
                                _ => {
                                    assert_eq!(
                                        args.len(),
                                        2,
                                        "Unexpected number of arguments for `{name}`"
                                    );
                                    assert!(matches!(
                                        args[0].node.ty(&self.body, self.tcx).kind(),
                                        TyKind::RawPtr(_, Mutability::Mut)
                                    ));
                                    let src_set = self.follow_rvalue(state, args[1].node.clone());
                                    let dst_set = self.follow_deref(state, args[0].node.clone());
                                    let destination_set =
                                        state.follow_from_place(*destination, self.def_id);
                                    state.extend(&destination_set, &state.follow(&dst_set));
                                    state.extend(&dst_set, &src_set);
                                }
                            };
                        }
                        // Similar to `copy_nonoverlapping`, argument order is `src`, `dst`, `count`.
                        "copy" => {
                            assert_eq!(args.len(), 3, "Unexpected number of arguments for `copy`");
                            assert!(matches!(
                                args[0].node.ty(&self.body, self.tcx).kind(),
                                TyKind::RawPtr(_, Mutability::Not)
                            ));
                            assert!(matches!(
                                args[1].node.ty(&self.body, self.tcx).kind(),
                                TyKind::RawPtr(_, Mutability::Mut)
                            ));
                            self.apply_copy_effect(
                                state,
                                args[0].node.clone(),
                                args[1].node.clone(),
                            );
                        }
                        // Similar to `copy_nonoverlapping`, argument order is `dst`, `src`, `count`.
                        "volatile_copy_memory" | "volatile_copy_nonoverlapping_memory" => {
                            assert_eq!(args.len(), 3, "Unexpected number of arguments for `copy`");
                            assert!(matches!(
                                args[0].node.ty(&self.body, self.tcx).kind(),
                                TyKind::RawPtr(_, Mutability::Mut)
                            ));
                            assert!(matches!(
                                args[1].node.ty(&self.body, self.tcx).kind(),
                                TyKind::RawPtr(_, Mutability::Not)
                            ));
                            self.apply_copy_effect(
                                state,
                                args[1].node.clone(),
                                args[0].node.clone(),
                            );
                        }
                        // Semantically equivalent to dest = *a
                        "volatile_load" | "unaligned_volatile_load" => {
                            assert_eq!(
                                args.len(),
                                1,
                                "Unexpected number of arguments for `volatile_load`"
                            );
                            assert!(matches!(
                                args[0].node.ty(&self.body, self.tcx).kind(),
                                TyKind::RawPtr(_, Mutability::Not)
                            ));
                            // Destination of the return value.
                            let lvalue_set = state.follow_from_place(*destination, self.def_id);
                            let rvalue_set = self.follow_deref(state, args[0].node.clone());
                            state.extend(&lvalue_set, &state.follow(&rvalue_set));
                        }
                        // Semantically equivalent *a = b.
                        "volatile_store" | "unaligned_volatile_store" => {
                            assert_eq!(
                                args.len(),
                                2,
                                "Unexpected number of arguments for `volatile_store`"
                            );
                            assert!(matches!(
                                args[0].node.ty(&self.body, self.tcx).kind(),
                                TyKind::RawPtr(_, Mutability::Mut)
                            ));
                            let lvalue_set = self.follow_deref(state, args[0].node.clone());
                            let rvalue_set = self.follow_rvalue(state, args[1].node.clone());
                            state.extend(&lvalue_set, &rvalue_set);
                        }
                        _ => {
                            // TODO: go through the list of intrinsics and make sure none have
                            // slipped; I am sure we still missing some.
                            if self.tcx.is_mir_available(def_id) {
                                self.apply_regular_call_effect(state, instance, args, destination);
                            }
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
                                let lvalue_set = state.follow_from_place(*destination, self.def_id);
                                let rvalue_set = HashSet::from([
                                    LocalMemLoc::new_alloc().with_def_id(self.def_id)
                                ]);
                                state.extend(&lvalue_set, &rvalue_set);
                            }
                            _ => {}
                        }
                    } else {
                        self.apply_regular_call_effect(state, instance, args, destination);
                    }
                }
            }
        };
        terminator.edges()
    }

    // We probably should not care about this.
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
            match Instance::try_resolve(tcx, ParamEnv::reveal_all(), *def, &args) {
                Ok(Some(instance)) => Ok(instance),
                _ => Err(format!("Kani does not support reasoning about arguments to `{ty:?}`.")),
            }
        }
        _ => Err(format!("Kani does not support reasoning about arguments to `{ty:?}`.")),
    }
}

impl<'a, 'tcx> PointsToAnalysis<'a, 'tcx> {
    // Update the analysis state according to the operation, which is semantically equivalent to `*to = *from`.
    fn apply_copy_effect(
        &self,
        state: &mut PointsToGraph<'tcx>,
        from: Operand<'tcx>,
        to: Operand<'tcx>,
    ) {
        let lvalue_set = self.follow_deref(state, to);
        let rvalue_set = self.follow_deref(state, from);
        state.extend(&lvalue_set, &state.follow(&rvalue_set));
    }

    // Find all places where the operand could point to at the current stage of the program.
    fn follow_rvalue(
        &self,
        state: &mut PointsToGraph<'tcx>,
        operand: Operand<'tcx>,
    ) -> HashSet<GlobalMemLoc<'tcx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                // Find all places which are pointed to by the place.
                state.follow(&state.follow_from_place(place, self.def_id))
            }
            Operand::Constant(const_operand) => {
                // Constants could point to a static, so need to check for that.
                if let Some(static_def_id) = const_operand.check_static_ptr(self.tcx) {
                    HashSet::from([GlobalMemLoc::Global(static_def_id)])
                } else {
                    HashSet::new()
                }
            }
        }
    }

    // Find all places where the deref of the operand could point to at the current stage of the program.
    fn follow_deref(
        &self,
        state: &mut PointsToGraph<'tcx>,
        operand: Operand<'tcx>,
    ) -> HashSet<GlobalMemLoc<'tcx>> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => state.follow_from_place(
                place.project_deeper(&[ProjectionElem::Deref], self.tcx),
                self.def_id,
            ),
            Operand::Constant(const_operand) => {
                // Constants could point to a static, so need to check for that.
                if let Some(static_def_id) = const_operand.check_static_ptr(self.tcx) {
                    HashSet::from([GlobalMemLoc::Global(static_def_id)])
                } else {
                    HashSet::new()
                }
            }
        }
    }

    // Update the analysis state according to the regular function call.
    fn apply_regular_call_effect(
        &mut self,
        state: &mut PointsToGraph<'tcx>,
        instance: Instance,
        args: &[Spanned<Operand<'tcx>>],
        destination: &Place<'tcx>,
    ) {
        // Here we simply call another function, so need to retrieve internal
        // body for it.
        let new_body = {
            let internal_instance = rustc_internal::stable(instance);
            assert!(self.instances.contains(&internal_instance));
            let stable_body = self.transformer.body(self.tcx, rustc_internal::stable(instance));
            stable_body.internal_mir(self.tcx)
        };

        // Recursively run the analysis and join the results into the current state.
        let new_result = PointsToAnalysis::run(
            new_body,
            self.tcx,
            instance.def_id(),
            self.call_graph,
            self.instances,
            self.transformer,
        );
        state.join(&new_result);

        // One missing link is the connections between the arguments in the
        // caller and parameters in the callee, add it to the graph.
        //
        // TODO: this is probably wrong if the arguments are passed via spread,
        // as in with closures, so we would need to fix that.
        for (i, arg) in args.iter().enumerate() {
            let lvalue_set = HashSet::from([LocalMemLoc::Place(Place {
                local: (i + 1).into(), // Since arguments in the callee are starting with 1, account for that.
                projection: List::empty(),
            })
            .with_def_id(instance.def_id())]);
            let rvalue_set = self.follow_rvalue(state, arg.node.clone());
            state.extend(&lvalue_set, &rvalue_set);
        }
        // Similarly, need to connect the return value to the return
        // destination.
        let lvalue_set = state.follow_from_place(*destination, self.def_id);
        let rvalue_set = HashSet::from([LocalMemLoc::Place(Place {
            local: 0usize.into(),
            projection: List::empty(),
        })
        .with_def_id(instance.def_id())]);
        state.extend(&lvalue_set, &state.follow(&rvalue_set));
    }
}
