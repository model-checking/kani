// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains an interface for setting custom query implementations
//! to run during code generation. For example, this can be used to hook up
//! custom MIR transformations.

use crate::args::{Arguments, ReachabilityType};
use crate::kani_middle::intrinsics::ModelIntrinsics;
use crate::kani_queries::QueryDb;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::util::Providers;
use rustc_middle::{mir::Body, ty::TyCtxt};

/// Sets up rustc's query mechanism to apply Kani's custom queries to code from
/// a crate.
pub fn provide(providers: &mut Providers, queries: &QueryDb) {
    let args = queries.args();
    if should_override(args) {
        // Don't override queries if we are only compiling our dependencies.
        providers.optimized_mir = run_mir_passes;
        providers.extern_queries.optimized_mir = run_mir_passes_extern;
    }
}

fn should_override(args: &Arguments) -> bool {
    args.reachability_analysis != ReachabilityType::None && !args.build_std
}

/// Returns the optimized code for the external function associated with `def_id` by
/// running rustc's optimization passes followed by Kani-specific passes.
fn run_mir_passes_extern(tcx: TyCtxt, def_id: DefId) -> &Body {
    tracing::debug!(?def_id, "run_mir_passes_extern");
    let body = (rustc_interface::DEFAULT_QUERY_PROVIDERS.extern_queries.optimized_mir)(tcx, def_id);
    run_kani_mir_passes(tcx, def_id, body)
}

/// Returns the optimized code for the local function associated with `def_id` by
/// running rustc's optimization passes followed by Kani-specific passes.
fn run_mir_passes(tcx: TyCtxt, def_id: LocalDefId) -> &Body {
    tracing::debug!(?def_id, "run_mir_passes");
    let body = (rustc_interface::DEFAULT_QUERY_PROVIDERS.optimized_mir)(tcx, def_id);
    run_kani_mir_passes(tcx, def_id.to_def_id(), body)
}

/// Returns the optimized code for the function associated with `def_id` by
/// running Kani-specific passes. The argument `body` should be the optimized
/// code rustc generates for this function.
fn run_kani_mir_passes<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    body: &'tcx Body<'tcx>,
) -> &'tcx Body<'tcx> {
    tracing::debug!(?def_id, "Run Kani transformation passes");
    let mut transformed_body = body.clone();
    // This should be applied after stubbing so user stubs take precedence.
    ModelIntrinsics::run_pass(tcx, &mut transformed_body);
    tcx.arena.alloc(transformed_body)
}
