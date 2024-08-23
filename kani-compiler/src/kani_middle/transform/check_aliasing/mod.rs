// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a pass that instruments code with assertions
//! that will fail when the aliasing model is violated.

// Reimport components of mir that conflict with
// parts of the sub-pass's API.
pub use stable_mir::Error as MirError;
pub use stable_mir::mir::mono::Instance as MirInstance;


mod actions;
use actions::*;
mod function_cache;
use function_cache::*;
mod instrumentation;
use instrumentation::*;

use crate::args::ExtraChecks;
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use stable_mir::mir::Body;
use rustc_middle::ty::TyCtxt;
use std::fmt::Debug;
use tracing::trace;

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
#[derive(Debug, Default)]
pub struct AliasingPass {
    cache: Cache,
}

impl AliasingPass {
    pub fn new() -> AliasingPass {
        Default::default()
    }
}

/// Functions containing any of the following in their
/// prefix or in their name will be ignored.
/// This allows skipping instrumenting functions that
/// are called by the instrumentation functions.
const ALIASING_BLACKLIST: &'static [&'static str] = &[
    "kani",              // Skip kani functions
    "std::mem::size_of", // skip size_of::<T>
    "core::num",         // Skip numerical ops (like .wrapping_add)
    "std::ptr",          // Skip pointer manipulation functions
    "KaniInitializeSState",
    "KaniInitializeLocal",
    "KaniStackCheckPtr",
    "KaniStackCheckRef",
    "KaniNewMutRefFromValue",
    "KaniNewMutRawFromRef",
    "KaniNewMutRefFromRaw",
    "std::array",
    "std::ops",
    "core::panicking",
    "std::rt",
    "std::panic",
    "core::panic",
    "std::fmt",
    "std::iter",
    "core::ub_checks",
    "std::cmp",
    "core::slice",
    "std::mem",
    // This blacklist needs expansion.
];

// Currently, the above list of functions is too
// coarse-grained; because all kani functions
// are skipped, many std modules are skipped,
// and kani functions are skipped, this pass
// cannot be used to verify functions
// in those modules, despite the fact that
// only some of those functions in those modules
// are called by the instrumented code.

impl TransformPass for AliasingPass {
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
        let args = query_db.args();
        args.ub_check.contains(&ExtraChecks::Aliasing)
    }

    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: MirInstance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform: aliasing pass");
        if ALIASING_BLACKLIST
            .iter()
            .fold(false, |blacklisted, member| blacklisted || instance.name().contains(member))
        {
            (false, body)
        } else {
            // let body = CachedBodyMutator::from(body);
            let mut instrumentation_data = InstrumentationData::new(tcx, &mut self.cache, body);
            // let out = BodyMutationPassState::new(instrumentation_data).finalize();
            instrumentation_data.instrument_locals().unwrap();
            instrumentation_data.instrument_instructions().unwrap();
            (true, instrumentation_data.finalize().into())
        }
    }
}
