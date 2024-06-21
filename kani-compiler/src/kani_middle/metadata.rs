// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles Kani metadata generation. For example, generating HarnessMetadata for a
//! given function.

use std::path::Path;

use crate::kani_middle::attributes::test_harness_name;
use kani_metadata::{ArtifactType, HarnessAttributes, HarnessMetadata};
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::CrateDef;

use super::{attributes::KaniAttributes, SourceLocation};

/// Create the harness metadata for a proof harness for a given function.
pub fn gen_proof_metadata(tcx: TyCtxt, instance: Instance, base_name: &Path) -> HarnessMetadata {
    let def = instance.def;
    let kani_attributes = KaniAttributes::for_instance(tcx, instance);
    let pretty_name = instance.name();
    let mangled_name = instance.mangled_name();

    // We get the body span to include the entire function definition.
    // This is required for concrete playback to properly position the generated test.
    let loc = SourceLocation::new(instance.body().unwrap().span);
    let file_stem = format!("{}_{mangled_name}", base_name.file_stem().unwrap().to_str().unwrap());
    let model_file = base_name.with_file_name(file_stem).with_extension(ArtifactType::SymTabGoto);

    HarnessMetadata {
        pretty_name,
        mangled_name,
        crate_name: def.krate().name,
        original_file: loc.filename,
        original_start_line: loc.start_line,
        original_end_line: loc.end_line,
        attributes: kani_attributes.harness_attributes(),
        // TODO: This no longer needs to be an Option.
        goto_file: Some(model_file),
        contract: Default::default(),
    }
}

/// Create the harness metadata for a test description.
#[allow(dead_code)]
pub fn gen_test_metadata(
    tcx: TyCtxt,
    test_desc: impl CrateDef,
    test_fn: Instance,
    base_name: &Path,
) -> HarnessMetadata {
    let pretty_name = test_harness_name(tcx, &test_desc);
    let mangled_name = test_fn.mangled_name();
    let loc = SourceLocation::new(test_desc.span());
    let file_stem = format!("{}_{mangled_name}", base_name.file_stem().unwrap().to_str().unwrap());
    let model_file = base_name.with_file_name(file_stem).with_extension(ArtifactType::SymTabGoto);

    HarnessMetadata {
        pretty_name,
        mangled_name,
        crate_name: test_desc.krate().name,
        original_file: loc.filename,
        original_start_line: loc.start_line,
        original_end_line: loc.end_line,
        attributes: HarnessAttributes::default(),
        // TODO: This no longer needs to be an Option.
        goto_file: Some(model_file),
        contract: Default::default(),
    }
}
