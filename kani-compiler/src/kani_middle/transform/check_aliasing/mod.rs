// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a pass that instruments code with assertions
//! that will fail when the aliasing model is violated.

use stable_mir::mir::mono::MonoItem;
use stable_mir::CrateDef;
// Reimport components of mir that conflict with
// parts of the sub-pass's API.
pub use stable_mir::mir::mono::Instance as MirInstance;
pub use stable_mir::Error as MirError;

mod actions;
use actions::*;
mod function_cache;
use function_cache::*;
mod instrumentation;
use instrumentation::*;

use crate::args::ExtraChecks;
use crate::kani_middle::reachability::{collect_reachable_items};
use crate::kani_middle::transform::{TransformPass, TransformationResult, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::Body;
use std::collections::HashSet;
use std::fmt::Debug;
use tracing::trace;

use super::GlobalPass;

/// Instrument the code with checks for aliasing model
/// violations.
/// Cache functions in-between applications of the pass.
/// Architecturally, this is implemented as the composition
/// of several sub passes on functions:
/// First, information is collected on the variables in the
/// function body and on the arguments to the function.
/// (LocalCollectionPassState)
/// Then, enough information from the body
/// is collected for instrumentation.
///
/// The body is transformed into a CachedBodyMutator to
/// be used in the BodyMutationPass, which combines the
/// body with (initially empty) storage for
/// instrumented locals and instrumented instructions,
/// and which caches function items referring to
/// resolved function instances.
///
/// The prologue of the function is then instrumented with data for every
/// stack allocation referenced by a local (instrument_locals).
/// Pointers to these locals are kept in InstrumentationData,
/// which then checks all instructions that modify memory for
/// aliasing violations (instrument_instructions).
///
/// Finally, a new body is made from the code + the instrumented
/// code.
#[derive(Debug)]
struct AliasingPass<'cache> {
    cache: &'cache mut Cache,
}

impl<'cache> TransformPass for AliasingPass<'cache> {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Instrumentation
    }

    fn is_enabled(&self, _query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: MirInstance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform: aliasing pass");
        // let body = CachedBodyMutator::from(body);
        let mut instrumentation_data = InstrumentationData::new(tcx, &mut self.cache, body);
        // let out = BodyMutationPassState::new(instrumentation_data).finalize();
        instrumentation_data.instrument_locals().unwrap();
        instrumentation_data.instrument_instructions().unwrap();
        (true, instrumentation_data.finalize().into())
    }
}

/// The global aliasing pass keeps a cache of resolved generic functions,
/// and ensures that only the functions that are called
/// from the proof harness itself are instrumented.
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
        let args = query_db.args();
        args.ub_check.contains(&ExtraChecks::Aliasing)
    }

    fn transform(
        &mut self,
        tcx: TyCtxt,
        _call_graph: &crate::kani_middle::reachability::CallGraph,
        _starting_items: &[stable_mir::mir::mono::MonoItem],
        instances: Vec<MirInstance>,
        transformer: &mut super::BodyTransformation,
    ) {
        let mut found = HashSet::new();
        // Collect
        for instance in &instances {
            if instance.def.all_attrs().into_iter().fold(false, |is_proof, attr| is_proof || attr.as_str().contains("kanitool::proof")) {
                let (items, _) = collect_reachable_items(tcx, transformer, &[MonoItem::Fn(*instance)]);
                for item in items {
                    if let MonoItem::Fn(instance) = item {
                        found.insert(instance);
                    }
                }
            }
        }
        eprintln!("Found is: {:?}", found);
        // Instrument
        for instance in &instances {
            if found.contains(instance) {
                found.remove(instance);
                let mut pass = AliasingPass { cache: &mut self.cache };
                let (_, body) =
                    pass.transform(tcx, transformer.body(tcx, *instance), *instance);
                transformer
                    .cache
                    .insert(*instance, TransformationResult::Modified(body));
            }
        }
    }
}
