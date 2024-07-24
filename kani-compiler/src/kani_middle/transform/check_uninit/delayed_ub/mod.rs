// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Global transformation pass that injects checks that catch delayed UB caused by uninitialized memory.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::args::ExtraChecks;
use crate::kani_middle::reachability::CallGraph;
use crate::kani_middle::transform::body::CheckType;
use crate::kani_middle::transform::body::MutableBody;
use crate::kani_middle::transform::check_uninit::UninitInstrumenter;
use crate::kani_middle::transform::internal_mir::RustcInternalMir;
use crate::kani_middle::transform::BodyTransformation;
use crate::kani_middle::transform::GlobalPass;
use crate::kani_middle::transform::TransformationResult;
use crate::kani_queries::QueryDb;
use initial_target_visitor::AnalysisTarget;
use initial_target_visitor::InitialTargetVisitor;
use instrumentation_visitor::InstrumentationVisitor;
use points_to_analysis::PointsToAnalysis;
use points_to_graph::GlobalMemLoc;
use points_to_graph::LocalMemLoc;
use points_to_graph::PointsToGraph;
use rustc_middle::ty::TyCtxt;
use rustc_mir_dataflow::JoinSemiLattice;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::MirVisitor;
use stable_mir::ty::FnDef;
use stable_mir::CrateDef;

mod initial_target_visitor;
mod instrumentation_visitor;
mod points_to_analysis;
mod points_to_graph;

#[derive(Debug)]
pub struct DelayedUbPass {
    pub check_type: CheckType,
    pub mem_init_fn_cache: HashMap<&'static str, FnDef>,
}

impl DelayedUbPass {
    pub fn new(check_type: CheckType) -> Self {
        Self { check_type, mem_init_fn_cache: HashMap::new() }
    }
}

impl GlobalPass for DelayedUbPass {
    fn is_enabled(&self, query_db: &QueryDb) -> bool {
        let args = query_db.args();
        args.ub_check.contains(&ExtraChecks::Uninit)
    }

    fn transform(
        &mut self,
        tcx: TyCtxt,
        call_graph: &CallGraph,
        starting_items: &[MonoItem],
        instances: Vec<Instance>,
        transformer: &mut BodyTransformation,
    ) {
        // Collect all analysis targets (pointers to places reading and writing from which should be
        // tracked).
        let targets: HashSet<_> = instances
            .iter()
            .flat_map(|instance| {
                let def_id = rustc_internal::internal(tcx, instance.def.def_id());
                let body = instance.body().unwrap();
                let mut visitor = InitialTargetVisitor::new(body.clone());
                visitor.visit_body(&body);
                // Convert all places into the format of aliasing graph for later comparison.
                visitor.into_targets().into_iter().map(move |analysis_target| match analysis_target
                {
                    AnalysisTarget::Place(place) => {
                        LocalMemLoc::Place(rustc_internal::internal(tcx, place)).with_def_id(def_id)
                    }
                    AnalysisTarget::Static(def_id) => {
                        GlobalMemLoc::Global(rustc_internal::internal(tcx, def_id))
                    }
                })
            })
            .collect();

        // Only perform this analysis if there is something to analyze.
        if !targets.is_empty() {
            let mut analysis_targets = HashSet::new();
            let mut global_points_to_graph = PointsToGraph::empty();
            // Analyze aliasing for every harness.
            for entry_item in starting_items {
                // Convert each entry function into instance, if possible.
                let entry_fn = match entry_item {
                    MonoItem::Fn(instance) => Some(*instance),
                    MonoItem::Static(static_def) => {
                        let instance: Instance = (*static_def).into();
                        instance.has_body().then_some(instance)
                    }
                    MonoItem::GlobalAsm(_) => None,
                };
                if let Some(instance) = entry_fn {
                    let body = instance.body().unwrap();
                    // Dataflow analysis does not yet work with StableMIR, so need to perform backward
                    // conversion.
                    let internal_body = body.internal_mir(tcx);
                    let internal_def_id = rustc_internal::internal(tcx, instance.def.def_id());
                    let results = PointsToAnalysis::run(
                        internal_body.clone(),
                        tcx,
                        internal_def_id,
                        call_graph,
                        &instances,
                        transformer,
                        &PointsToGraph::empty(),
                    );
                    // Since analysis targets are *pointers*, need to get its followers for instrumentation.
                    for target in targets.iter() {
                        analysis_targets.extend(results.pointees_of(target));
                    }
                    global_points_to_graph.join(&results);
                }
            }

            // Instrument each instance based on the final targets we found.
            for instance in instances {
                let internal_def_id = rustc_internal::internal(tcx, instance.def.def_id());
                let mut instrumenter = UninitInstrumenter {
                    check_type: self.check_type.clone(),
                    mem_init_fn_cache: &mut self.mem_init_fn_cache,
                };
                // Retrieve the body with all local instrumentation passes applied.
                let body = MutableBody::from(transformer.body(tcx, instance));
                // Instrument for delayed UB.
                let target_finder = InstrumentationVisitor::new(
                    &global_points_to_graph,
                    &analysis_targets,
                    internal_def_id,
                    tcx,
                );
                let (instrumentation_added, body) =
                    instrumenter.instrument(tcx, body, instance, target_finder);
                // If some instrumentation has been performed, update the cached body in the local transformer.
                if instrumentation_added {
                    transformer.cache.entry(instance).and_modify(|transformation_result| {
                        *transformation_result = TransformationResult::Modified(body.into());
                    });
                }
            }
        }
    }
}
