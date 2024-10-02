// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles Kani metadata generation. For example, generating HarnessMetadata for a
//! given function.

use std::collections::HashMap;
use std::path::Path;

use crate::kani_middle::attributes::{KaniAttributes, test_harness_name};
use crate::kani_middle::{InternalDefId, SourceLocation};
use kani_metadata::ContractedFunction;
use kani_metadata::{ArtifactType, HarnessAttributes, HarnessKind, HarnessMetadata};
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::{CrateDef, CrateItems};

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

/// Collects contract and contract harness metadata.
///
/// For each function with contracts (or that is a target of a contract harness),
/// construct a ContractedFunction object for it.
pub fn gen_contracts_metadata(tcx: TyCtxt) -> Vec<ContractedFunction> {
    // We work with stable_mir::CrateItem instead of stable_mir::Instance to include generic items
    let crate_items: CrateItems = stable_mir::all_local_items();

    let mut fn_to_data: HashMap<InternalDefId, ContractedFunction> = HashMap::new();

    for item in crate_items {
        let internal_def_id = rustc_internal::internal(tcx, item.def_id());

        let function = item.name();
        let file = SourceLocation::new(item.span()).filename;
        let attributes = KaniAttributes::for_def_id(tcx, item.def_id());

        if attributes.has_contract() {
            fn_to_data.insert(internal_def_id, ContractedFunction {
                function,
                file,
                harnesses: vec![],
            });
        } else if let Some((target_name, target_def_id, _)) =
            attributes.interpret_for_contract_attribute()
        {
            if let Some(cf) = fn_to_data.get_mut(&target_def_id) {
                cf.harnesses.push(function);
            } else {
                fn_to_data.insert(target_def_id, ContractedFunction {
                    function: target_name.to_string(),
                    file,
                    harnesses: vec![function],
                });
            }
        }
    }

    fn_to_data.into_values().collect()
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
        attributes: HarnessAttributes::new(HarnessKind::Test),
        // TODO: This no longer needs to be an Option.
        goto_file: Some(model_file),
        contract: Default::default(),
    }
}
