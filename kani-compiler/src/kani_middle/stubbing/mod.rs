// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for implementing stubbing.

mod annotations;
mod transform;

use std::collections::HashMap;

use kani_metadata::HarnessMetadata;
use rustc_hir::def_id::DefId;
use rustc_hir::definitions::DefPathHash;
use rustc_middle::ty::TyCtxt;
pub use transform::*;

use self::annotations::update_stub_mapping;

/// Collects the stubs from the harnesses in a crate.
pub fn harness_stub_map(
    tcx: TyCtxt,
    harness: DefId,
    metadata: &HarnessMetadata,
) -> HashMap<DefPathHash, DefPathHash> {
    let attrs = &metadata.attributes;
    let mut stub_pairs = HashMap::default();
    for stubs in &attrs.stubs {
        update_stub_mapping(tcx, harness.expect_local(), stubs, &mut stub_pairs);
    }
    stub_pairs
}
