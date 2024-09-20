// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Collects contract and contract harness metadata for the list subcommand.

use std::collections::HashMap;

use crate::kani_middle::attributes::{matches_diagnostic as matches_function, KaniAttributes};
use crate::kani_middle::{find_closure_in_body, InternalDefId, SourceLocation};
use kani_metadata::ContractedFunction;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Body, TerminatorKind};
use stable_mir::ty::{RigidTy, TyKind};
use stable_mir::{CrateDef, CrateItems};

/// Map each function to its contract harnesses
/// `fns` includes all functions with contracts and all functions that are targets of a contract harness.
fn fns_to_harnesses(tcx: TyCtxt) -> HashMap<InternalDefId, Vec<String>> {
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

/// For each function with contracts (or that is a target of a contract harness),
/// construct a ContractedFunction object for it and store it in `units`.
pub fn collect_contracted_fns(tcx: TyCtxt) -> Vec<ContractedFunction> {
    let mut contracted_fns = vec![];
    for (fn_def_id, harnesses) in fns_to_harnesses(tcx) {
        let attrs = KaniAttributes::for_item(tcx, fn_def_id);

        // It's possible that a function is a target of a proof for contract but does not actually have contracts.
        // If the function does have contracts, count them.
        let total_contracts = if attrs.has_contract() {
            let contract_attrs =
                KaniAttributes::for_item(tcx, fn_def_id).contract_attributes().unwrap();
            let body: Body = rustc_internal::stable(tcx.optimized_mir(fn_def_id));
            let check_body: Body =
                find_closure_in_body(&body, contract_attrs.checked_with.as_str())
                    .unwrap()
                    .body()
                    .unwrap();

            count_contracts(tcx, &check_body)
        } else {
            0
        };

        contracted_fns.push(ContractedFunction {
            function: tcx.def_path_str(fn_def_id),
            file: SourceLocation::new(rustc_internal::stable(tcx.def_span(fn_def_id))).filename,
            harnesses,
            total_contracts,
        });
    }

    contracted_fns
}
