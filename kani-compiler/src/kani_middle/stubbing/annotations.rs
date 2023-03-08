// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This file contains code for extracting stubbing-related attributes.

use kani_metadata::Stub;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::definitions::DefPathHash;
use rustc_middle::ty::TyCtxt;

use crate::kani_middle::attributes::extract_harness_attributes;
use crate::kani_middle::resolve::resolve_fn;

/// Collects the stubs from the harnesses in a crate, running rustc (to
/// expansion) with the supplied arguments `rustc_args`.
pub fn collect_stub_mappings(
    tcx: TyCtxt,
) -> FxHashMap<String, FxHashMap<DefPathHash, DefPathHash>> {
    tcx.hir_crate_items(())
        .items()
        .filter_map(|item| {
            let local_def_id = item.owner_id.def_id;
            let def_id = local_def_id.to_def_id();
            let attributes = extract_harness_attributes(tcx, def_id);
            // This currently runs before we validate all items. Abort if any error was found.
            tcx.sess.abort_if_errors();
            attributes.map(|attrs| {
                // TODO: Use collect instead.
                let mut stub_pairs = FxHashMap::default();
                for stubs in attrs.stubs {
                    update_stub_mapping(tcx, local_def_id, stubs, &mut stub_pairs);
                }
                let harness_name = tcx.def_path_str(def_id);
                (harness_name, stub_pairs)
            })
        })
        .collect()
}

/// Given a `kani::stub` attribute, tries to extract a pair of paths (the
/// original function/method, and its stub). Returns `None` and errors if the
/// attribute's arguments are not two paths.
fn stub_def_ids(tcx: TyCtxt, harness: LocalDefId, stub: Stub) -> Option<(DefId, DefId)> {
    // Resolve the attribute arguments to `DefId`s
    let current_module = tcx.parent_module_from_def_id(harness);
    let resolve = |name: &str| -> Option<DefId> {
        let maybe_resolved = resolve_fn(tcx, current_module, name);
        match maybe_resolved {
            Ok(def_id) => {
                tracing::debug!(?def_id, "Resolved {name} to {}", tcx.def_path_str(def_id));
                Some(def_id)
            }
            Err(err) => {
                tcx.sess
                    .span_err(tcx.def_span(harness), format!("failed to resolve `{name}`: {err}"));
                None
            }
        }
    };
    let orig = resolve(&stub.original);
    let stub = resolve(&stub.replacement);
    Some((orig?, stub?))
}

/// Updates the running map `stub_pairs` that maps a function/method to its
/// stub. Errors if a function/method is mapped more than once.
fn update_stub_mapping(
    tcx: TyCtxt,
    harness: LocalDefId,
    stub: Stub,
    stub_pairs: &mut FxHashMap<DefPathHash, DefPathHash>,
) {
    if let Some((orig_id, stub_id)) = stub_def_ids(tcx, harness, stub) {
        let orig_hash = tcx.def_path_hash(orig_id);
        let stub_hash = tcx.def_path_hash(stub_id);
        let other_opt = stub_pairs.insert(orig_hash, stub_hash);
        if let Some(other) = other_opt {
            if other != stub_hash {
                tcx.sess.span_err(
                    tcx.def_span(harness),
                    format!(
                        "duplicate stub mapping: {} mapped to {} and {}",
                        tcx.def_path_str(orig_id),
                        tcx.def_path_str(stub_id),
                        tcx.def_path_str(tcx.def_path_hash_to_def_id(other, &mut || panic!()))
                    ),
                );
            }
        }
    }
}
