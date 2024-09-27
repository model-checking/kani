// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module handles Kani metadata generation. For example, generating HarnessMetadata for a
//! given function.

use std::collections::HashMap;
use std::path::Path;

use crate::kani_middle::attributes::test_harness_name;
use crate::kani_middle::attributes::{
    ContractAttributes, KaniAttributes, matches_diagnostic as matches_function,
};
use crate::kani_middle::{InternalDefId, SourceLocation, find_closure_in_body};
use kani_metadata::ContractedFunction;
use kani_metadata::{ArtifactType, HarnessAttributes, HarnessKind, HarnessMetadata};
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Body, TerminatorKind};
use stable_mir::ty::{RigidTy, TyKind};
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

/// Map each function under contract to its contract harnesses
fn fns_to_contract_harnesses(tcx: TyCtxt) -> HashMap<InternalDefId, Vec<String>> {
    // We work with stable_mir::CrateItem instead of stable_mir::Instance to include generic items
    let crate_items: CrateItems = stable_mir::all_local_items();

    let mut fns_to_harnesses: HashMap<InternalDefId, Vec<String>> = HashMap::new();

    for item in crate_items {
        let def_id = rustc_internal::internal(tcx, item.def_id());
        let fn_name = tcx.def_path_str(def_id);
        let attributes = KaniAttributes::for_item(tcx, def_id);

        if attributes.has_contract() {
            fns_to_harnesses.insert(def_id, vec![]);
        } else if let Some((_, target_def_id, _)) = attributes.interpret_for_contract_attribute() {
            if let Some(harnesses) = fns_to_harnesses.get_mut(&target_def_id) {
                harnesses.push(fn_name);
            } else {
                fns_to_harnesses.insert(target_def_id, vec![fn_name]);
            }
        }
    }

    fns_to_harnesses
}

/// Count the number of contracts in `check_body`, where `check_body` is the body of the
/// kanitool::checked_with closure (c.f. kani_macros::sysroot::contracts).
/// In this closure, preconditions are denoted by kani::assume() calls and postconditions by kani::assert() calls.
/// The number of contracts is the number of times these functions are called inside the closure
fn count_contracts(tcx: TyCtxt, check_body: &Body) -> usize {
    let mut count = 0;

    for bb in &check_body.blocks {
        if let TerminatorKind::Call { ref func, .. } = bb.terminator.kind {
            let fn_ty = func.ty(check_body.locals()).unwrap();
            if let TyKind::RigidTy(RigidTy::FnDef(fn_def, args)) = fn_ty.kind() {
                if let Ok(instance) = Instance::resolve(fn_def, &args) {
                    // For each precondition or postcondition, increment the count
                    if matches_function(tcx, instance.def, "KaniAssume")
                        || matches_function(tcx, instance.def, "KaniAssert")
                    {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// Collects contract and contract harness metadata.
///
/// For each function with contracts (or that is a target of a contract harness),
/// construct a ContractedFunction object for it.
pub fn gen_contracts_metadata(tcx: TyCtxt) -> Vec<ContractedFunction> {
    let mut contracted_fns = vec![];
    for (fn_def_id, harnesses) in fns_to_contract_harnesses(tcx) {
        let attrs: ContractAttributes =
            KaniAttributes::for_item(tcx, fn_def_id).contract_attributes().unwrap();
        let body: Body = rustc_internal::stable(tcx.optimized_mir(fn_def_id));
        let check_body: Body =
            find_closure_in_body(&body, attrs.checked_with.as_str()).unwrap().body().unwrap();

        let total_contracts = count_contracts(tcx, &check_body);

        contracted_fns.push(ContractedFunction {
            function: tcx.def_path_str(fn_def_id),
            file: SourceLocation::new(rustc_internal::stable(tcx.def_span(fn_def_id))).filename,
            harnesses,
            total_contracts,
        });
    }

    contracted_fns
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
