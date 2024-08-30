// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a pass that instruments code with assertions
//! that will fail when the aliasing model is violated.

use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::CrateDef;
// Reimport components of mir that conflict with
// parts of the sub-pass's API.
pub use stable_mir::Error as MirError;

mod visitor;
use visitor::*;
mod instrumentation;
use instrumentation::*;

use crate::args::ExtraChecks;
use crate::kani_middle::transform::{TransformPass, TransformationResult, TransformationType};
use crate::kani_middle::FnDefCache as Cache;
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::Body;
use std::collections::{HashSet, VecDeque};
use std::fmt::Debug;
use tracing::trace;

use super::GlobalPass;

/// Instrument the code with checks for aliasing model
/// violations.
/// Cache functions in-between applications of the pass.
/// This is performed by taking the incoming body,
/// using a visitor to find instructions relevant to
/// the instrumentation, then iterating over these
/// instructions backwards, inserting code prior to their
/// execution.
#[derive(Debug)]
struct AliasingPass<'cache> {
    cache: &'cache mut Cache,
}

/// Returns whether ExtraChecks::Aliasing is included
/// in the command line arguments
fn db_includes_aliasing(query_db: &QueryDb) -> bool {
    query_db.args().ub_check.contains(&ExtraChecks::Aliasing)
}

impl<'cache> TransformPass for AliasingPass<'cache> {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Instrumentation
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        db_includes_aliasing(query_db)
    }

    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform: aliasing pass");
        let instrumentation_data = InstrumentationData::new(tcx, &mut self.cache, body);
        let out = instrumentation_data.finalize().unwrap().into();
        (true, out)
    }
}

/// The global aliasing pass keeps a cache of resolved generic functions,
/// and ensures that only the functions that are called
/// from the proof harness itself are instrumented.
/// To avoid instrumenting functions that were not present in the source,
/// but added in the instrumented code, this first collects the functions
/// present in the source, then instruments them.
#[derive(Debug, Default)]
pub struct GlobalAliasingPass {
    cache: Cache,
}

impl GlobalAliasingPass {
    pub fn new() -> Self {
        Default::default()
    }
}

impl GlobalPass for GlobalAliasingPass {
    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        db_includes_aliasing(query_db)
    }

    fn transform(
        &mut self,
        tcx: TyCtxt,
        call_graph: &crate::kani_middle::reachability::CallGraph,
        _starting_items: &[stable_mir::mir::mono::MonoItem],
        instances: Vec<Instance>,
        transformer: &mut super::BodyTransformation,
    ) {
        let mut found = HashSet::new();
        let mut queue = VecDeque::new();
        // Collect
        for instance in &instances {
            if instance
                .def
                .all_attrs()
                .into_iter()
                .any(|attr| attr.as_str().contains("kanitool::proof"))
                && found.insert(instance)
            {
                queue.push_back(instance)
            }
        }
        while let Some(instance) = queue.pop_front() {
            let mut pass = AliasingPass { cache: &mut self.cache };
            let (_, body) = pass.transform(tcx, transformer.body(tcx, *instance), *instance);
            transformer.cache.insert(*instance, TransformationResult::Modified(body));
            for node in call_graph.successors(MonoItem::Fn(*instance).clone()) {
                if let MonoItem::Fn(adjacent) = node {
                    if found.insert(adjacent) {
                        queue.push_back(adjacent);
                    }
                }
            }
        }
    }
}
