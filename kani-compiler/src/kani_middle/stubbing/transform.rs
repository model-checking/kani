// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass that performs the
//! stubbing of functions and methods. The primary function of the module is
//! `transform`, which takes the `DefId` of a function/method and returns the
//! body of its stub, if appropriate. The stub mapping it uses is set via rustc
//! arguments.

use lazy_static::lazy_static;
use regex::Regex;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::{mir::Body, ty::TyCtxt};

/// Returns the new body of a function/method if it has been stubbed out;
/// otherwise, returns `None`.
pub fn transform(tcx: TyCtxt, def_id: DefId) -> Option<&Body> {
    if let Some(mapping) = get_stub_mapping(tcx) {
        let name = tcx.def_path_str(def_id);
        if let Some(replacement) = mapping.get(&name) {
            if let Some(replacement_id) = get_def_id(tcx, replacement) {
                // TODO: We need to perform validation here (e.g., check that
                // the replacement is compatible with the original function).
                // <https://github.com/model-checking/kani/issues/1892>
                let new_body = tcx.optimized_mir(replacement_id).clone();
                return Some(tcx.arena.alloc(new_body));
            } else {
                tcx.sess
                    .span_err(tcx.def_span(def_id), format!("Unable to find stub: {replacement}"));
            };
        }
    }
    None
}

/// The prefix we will use when serializing the stub mapping as a rustc argument.
const RUSTC_ARG_PREFIX: &str = "kani_stubs=";

/// Serializes the stub mapping into a rustc argument.
pub fn mk_rustc_arg(stub_mapping: FxHashMap<String, String>) -> String {
    // Store our serialized mapping as a fake LLVM argument (safe to do since
    // LLVM will never see them).
    format!("-Cllvm-args='{RUSTC_ARG_PREFIX}{}'", serde_json::to_string(&stub_mapping).unwrap())
}

/// Deserializes the stub mapping from the rustc argument value.
fn deserialize_mapping(val: &str) -> FxHashMap<String, String> {
    serde_json::from_str(val).unwrap()
}

/// Retrieves the stub mapping from the compiler configuration.
fn get_stub_mapping(tcx: TyCtxt) -> Option<FxHashMap<String, String>> {
    // Use a static so that we compile the regex only once.
    lazy_static! {
        static ref RE: Regex = Regex::new(&format!("'{RUSTC_ARG_PREFIX}(.*)'")).unwrap();
    }
    for arg in &tcx.sess.opts.cg.llvm_args {
        if let Some(captures) = RE.captures(arg) {
            return Some(deserialize_mapping(captures.get(1).unwrap().as_str()));
        }
    }
    None
}

/// Tries to find the `DefId` of a function/method that matches the path `path`.
fn get_def_id(tcx: TyCtxt, path: &str) -> Option<DefId> {
    // TODO: This linear scan is potentially inefficient; we should find another
    // way of resolving the path to a `DefId`.
    // <https://github.com/model-checking/kani/issues/1894>
    tcx.iter_local_def_id().map(LocalDefId::to_def_id).find(|&id| tcx.def_path_str(id) == path)
}
