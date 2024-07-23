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
use delayed_ub_visitor::DelayedUbVisitor;
use instrumentation_visitor::DelayedUbTargetVisitor;
use points_to_analysis::PointsToAnalysis;
use points_to_graph::PlaceOrAlloc;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::MirVisitor;
use stable_mir::mir::Place;
use stable_mir::ty::FnDef;
use stable_mir::CrateDef;

mod delayed_ub_visitor;
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
                let mut visitor = DelayedUbVisitor::new(body.clone());
                visitor.visit_body(&body);
                // Convert all places into the format of aliasing graph for later comparison.
                visitor.into_targets().into_iter().map(move |place| {
                    PlaceOrAlloc::Place(rustc_internal::internal(tcx, place)).with_def_id(def_id)
                })
            })
            .collect();

        // Only perform this analysis if there is something to analyze.
        if !targets.is_empty() {
            let mut places_need_instrumentation = HashSet::new();
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
                    );
                    // Since analysis targets are *pointers*, need to get its followers for instrumentation.
                    for target in targets.iter() {
                        places_need_instrumentation.extend(results.pointees_of(target));
                    }
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
                let new_body = MutableBody::from(transformer.body(tcx, instance));
                // Retrieve all places we need to instrument in the appropriate format.
                let place_filter: Vec<Place> = places_need_instrumentation
                    .iter()
                    .filter(|place| {
                        // Make sure only places from the current instance are included.
                        place.has_def_id(internal_def_id)
                    })
                    .filter_map(|global_place_or_alloc| {
                        match global_place_or_alloc.without_def_id() {
                            PlaceOrAlloc::Alloc(_) => None, // Allocations cannot be read directly, so we need not worry about them.
                            PlaceOrAlloc::Place(place) => Some(rustc_internal::stable(place)), // Convert back to StableMIR.
                        }
                    })
                    .collect();
                // Finally, instrument.
                let (instrumentation_added, body) = instrumenter
                    .instrument::<DelayedUbTargetVisitor>(tcx, new_body, instance, &place_filter);
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
