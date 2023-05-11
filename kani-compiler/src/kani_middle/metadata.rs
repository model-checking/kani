// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles Kani metadata generation. For example, generating HarnessMetadata for a
//! given function.

use std::path::Path;

use kani_metadata::{ArtifactType, HarnessMetadata};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Instance, TyCtxt};

use super::{attributes::extract_harness_attributes, SourceLocation};

/// Create the harness metadata for a proof harness for a given function.
pub fn gen_proof_metadata(tcx: TyCtxt, def_id: DefId, base_name: &Path) -> HarnessMetadata {
    let attributes = extract_harness_attributes(tcx, def_id);
    let pretty_name = tcx.def_path_str(def_id);
    let mangled_name = tcx.symbol_name(Instance::mono(tcx, def_id)).to_string();
    let loc = SourceLocation::def_id_loc(tcx, def_id);
    let file_stem = format!("{}_{mangled_name}", base_name.file_stem().unwrap().to_str().unwrap());
    let model_file = base_name.with_file_name(file_stem).with_extension(ArtifactType::SymTabGoto);

    HarnessMetadata {
        pretty_name,
        mangled_name,
        crate_name: tcx.crate_name(def_id.krate).to_string(),
        original_file: loc.filename,
        original_start_line: loc.start_line,
        original_end_line: loc.end_line,
        attributes: attributes.unwrap_or_default(),
        // TODO: This no longer needs to be an Option.
        goto_file: Some(model_file),
    }
}
