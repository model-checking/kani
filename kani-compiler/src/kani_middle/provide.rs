// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains an interface for setting custom query implementations
//! to run during code generation. For example, this can be used to hook up
//! custom MIR transformations.

use crate::args::{Arguments, ReachabilityType};
use crate::kani_middle::intrinsics::ModelIntrinsics;
use crate::kani_middle::reachability::{collect_reachable_items, filter_crate_items};
use crate::kani_middle::stubbing;
use crate::kani_middle::transform::BodyTransformation;
use crate::kani_queries::QueryDb;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::util::Providers;
use rustc_middle::{mir::Body, query::queries, ty::TyCtxt};
use stable_mir::mir::mono::MonoItem;

/// Sets up rustc's query mechanism to apply Kani's custom queries to code from
/// a crate.
pub fn provide(providers: &mut Providers, queries: &QueryDb) {
    let args = queries.args();
    if should_override(args) {
        // Don't override queries if we are only compiling our dependencies.
        providers.optimized_mir = run_mir_passes;
        providers.extern_queries.optimized_mir = run_mir_passes_extern;
        if args.stubbing_enabled {
            // TODO: Check if there's at least one stub being applied.
            providers.collect_and_partition_mono_items = collect_and_partition_mono_items;
        }
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
    let mut transformed_body = stubbing::transform(tcx, def_id, body);
    stubbing::transform_foreign_functions(tcx, &mut transformed_body);
    // This should be applied after stubbing so user stubs take precedence.
    ModelIntrinsics::run_pass(tcx, &mut transformed_body);
    tcx.arena.alloc(transformed_body)
}

/// Runs a reachability analysis before running the default
/// `collect_and_partition_mono_items` query. The reachability analysis finds
/// trait mismatches introduced by stubbing and performs a graceful exit in
/// these cases. Left to its own devices, the default query panics.
/// This is an issue when compiling a library, since the crate metadata is
/// generated (using this query) before code generation begins (which is
/// when we normally run the reachability analysis).
fn collect_and_partition_mono_items(
    tcx: TyCtxt,
    key: (),
) -> queries::collect_and_partition_mono_items::ProvidedValue {
    rustc_smir::rustc_internal::run(tcx, || {
        let local_reachable =
            filter_crate_items(tcx, |_, _| true).into_iter().map(MonoItem::Fn).collect::<Vec<_>>();

        // We do not actually need the value returned here.
        collect_reachable_items(tcx, &mut BodyTransformation::dummy(), &local_reachable);
    })
    .unwrap();
    (rustc_interface::DEFAULT_QUERY_PROVIDERS.collect_and_partition_mono_items)(tcx, key)
}
