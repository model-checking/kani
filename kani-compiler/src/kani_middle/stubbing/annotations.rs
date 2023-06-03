// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This file contains code for extracting stubbing-related attributes.

use std::collections::HashMap;

use kani_metadata::Stub;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::definitions::DefPathHash;
use rustc_middle::ty::TyCtxt;

use crate::kani_middle::resolve::resolve_fn;

/// Given a `kani::stub` attribute, tries to extract a pair of paths (the
/// original function/method, and its stub). Returns `None` and errors if the
/// attribute's arguments are not two paths.
fn stub_def_ids(tcx: TyCtxt, harness: LocalDefId, stub: &Stub) -> Option<(DefId, DefId)> {
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
pub fn update_stub_mapping(
    tcx: TyCtxt,
    harness: LocalDefId,
    stub: &Stub,
    stub_pairs: &mut HashMap<DefPathHash, DefPathHash>,
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
