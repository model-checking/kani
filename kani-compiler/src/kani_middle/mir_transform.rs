// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains infrastructure for performing transformation passes on the MIR.

use crate::kani_middle::stubbing;
use rustc_hir::def_id::DefId;
use rustc_interface;
use rustc_middle::{
    mir::Body,
    ty::{query::ExternProviders, query::Providers, TyCtxt},
};

/// Returns the optimized code for the function associated with `def_id` by
/// running rustc's optimization passes followed by Kani-specific passes.
fn run_passes<'tcx, const EXTERN: bool>(tcx: TyCtxt<'tcx>, def_id: DefId) -> &Body<'tcx> {
    tracing::debug!(?def_id, "Run rustc transformation passes");
    let optimized_mir = if EXTERN {
        rustc_interface::DEFAULT_EXTERN_QUERY_PROVIDERS.optimized_mir
    } else {
        rustc_interface::DEFAULT_QUERY_PROVIDERS.optimized_mir
    };
    let body = optimized_mir(tcx, def_id);
    run_kani_passes(tcx, def_id, body)
}

/// Returns the optimized code for the function associated with `def_id` by
/// running Kani-specific passes. The argument `body` should be the optimized
/// code rustc generates for this function.
fn run_kani_passes<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    body: &'tcx Body<'tcx>,
) -> &'tcx Body<'tcx> {
    tracing::debug!(?def_id, "Run Kani transformation passes");
    stubbing::transform(tcx, def_id).unwrap_or(body)
}

/// Sets up rustc's query mechanism to apply Kani's passes to code from the present crate.
pub fn provide(providers: &mut Providers) {
    providers.optimized_mir = run_passes::<false>;
}

/// Sets up rustc's query mechanism to apply Kani's passes to code from external crates.
pub fn provide_extern(providers: &mut ExternProviders) {
    providers.optimized_mir = run_passes::<true>;
}
