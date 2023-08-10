// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles Kani metadata generation. For example, generating HarnessMetadata for a
//! given function.

use std::path::Path;

use crate::kani_middle::attributes::test_harness_name;
use kani_metadata::{ArtifactType, HarnessAttributes, HarnessMetadata};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Instance, InstanceDef, TyCtxt};

use super::{attributes::KaniAttributes, SourceLocation};

/// Create the harness metadata for a proof harness for a given function.
pub fn gen_proof_metadata(tcx: TyCtxt, def_id: DefId, base_name: &Path) -> HarnessMetadata {
    let attributes = KaniAttributes::for_item(tcx, def_id).harness_attributes();
    let pretty_name = tcx.def_path_str(def_id);
    let mut mangled_name = tcx.symbol_name(Instance::mono(tcx, def_id)).to_string();
    // Main function a special case in order to support `--function main`
    // TODO: Get rid of this: https://github.com/model-checking/kani/issues/2129
    if pretty_name == "main" {
        mangled_name = pretty_name.clone()
    };

    let body = tcx.instance_mir(InstanceDef::Item(def_id));
    let loc = SourceLocation::new(tcx, &body.span);
    let file_stem =
        format!("{}_{}", base_name.file_stem().unwrap().to_str().unwrap(), mangled_name);
    let model_file = base_name.with_file_name(file_stem).with_extension(ArtifactType::SymTabGoto);

    HarnessMetadata {
        pretty_name,
        mangled_name,
        crate_name: tcx.crate_name(def_id.krate).to_string(),
        original_file: loc.filename,
        original_start_line: loc.start_line,
        original_end_line: loc.end_line,
        attributes,
        // TODO: This no longer needs to be an Option.
        goto_file: Some(model_file),
    }
}

/// Create the harness metadata for a test description.
#[allow(dead_code)]
pub fn gen_test_metadata<'tcx>(
    tcx: TyCtxt<'tcx>,
    test_desc: DefId,
    test_fn: Instance<'tcx>,
    base_name: &Path,
) -> HarnessMetadata {
    let pretty_name = test_harness_name(tcx, test_desc);
    let mangled_name = tcx.symbol_name(test_fn).to_string();
    let body = tcx.instance_mir(InstanceDef::Item(test_desc));
    let loc = SourceLocation::new(tcx, &body.span);
    let file_stem = format!("{}_{mangled_name}", base_name.file_stem().unwrap().to_str().unwrap());
    let model_file = base_name.with_file_name(file_stem).with_extension(ArtifactType::SymTabGoto);

    HarnessMetadata {
        pretty_name,
        mangled_name,
        crate_name: tcx.crate_name(test_desc.krate).to_string(),
        original_file: loc.filename,
        original_start_line: loc.start_line,
        original_end_line: loc.end_line,
        attributes: HarnessAttributes::default(),
        // TODO: This no longer needs to be an Option.
        goto_file: Some(model_file),
    }
}
