// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This file contains code for extracting stubbing-related attributes.

use rustc_ast::Attribute;
use rustc_data_structures::fx::FxHashMap;
use rustc_driver::RunCompiler;
use rustc_driver::{Callbacks, Compilation};
use rustc_errors::ErrorGuaranteed;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::definitions::DefPathHash;
use rustc_interface::interface::Compiler;
use rustc_interface::Queries;
use rustc_middle::ty::TyCtxt;

use crate::kani_middle::attributes::{extract_path_arguments, partition_kanitool_attributes};
use crate::kani_middle::resolve::resolve_path;

/// Collects the stubs from the harnesses in a crate, running rustc (to
/// expansion) with the supplied arguments `rustc_args`.
pub fn collect_stub_mappings(
    rustc_args: &[String],
) -> Result<FxHashMap<String, FxHashMap<DefPathHash, DefPathHash>>, ErrorGuaranteed> {
    let mut callbacks = CollectorCallbacks::default();
    let compiler = RunCompiler::new(rustc_args, &mut callbacks);
    compiler.run().map(|_| callbacks.stub_mapping)
}

/// A rustc callback that is used to collect the stub mappings specified for
/// each harness.
#[derive(Default)]
struct CollectorCallbacks {
    stub_mapping: FxHashMap<String, FxHashMap<DefPathHash, DefPathHash>>,
}

impl Callbacks for CollectorCallbacks {
    /// The main callback, invoked after the HIR has been created.
    fn after_expansion<'tcx>(
        &mut self,
        _compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
            for item in tcx.hir_crate_items(()).items() {
                let local_def_id = item.owner_id.def_id;
                let def_id = local_def_id.to_def_id();
                let (proof, other) = partition_kanitool_attributes(tcx.get_attrs_unchecked(def_id));
                // Ignore anything that is not a harness
                if proof.is_empty() {
                    continue;
                }
                let mut stub_pairs = FxHashMap::default();
                for (name, attr) in other {
                    if name == "stub" {
                        update_stub_mapping(tcx, local_def_id, attr, &mut stub_pairs);
                    }
                }
                let harness_name = tcx.def_path_str(def_id);
                self.stub_mapping.insert(harness_name, stub_pairs);
            }
            tcx.sess.abort_if_errors();
            // We do not need to continue compilation after we've collected the stub mappings
            Compilation::Stop
        })
    }
}

/// Given a `kani::stub` attribute, tries to extract a pair of paths (the
/// original function/method, and its stub). Returns `None` and errors if the
/// attribute's arguments are not two paths.
fn extract_stubbing_pair(
    tcx: TyCtxt,
    harness: LocalDefId,
    attr: &Attribute,
) -> Option<(DefId, DefId)> {
    // Extract the attribute arguments
    let args = extract_path_arguments(attr);
    if args.len() != 2 {
        tcx.sess.span_err(
            attr.span,
            format!("Attribute `kani::stub` takes two path arguments; found {}", args.len()),
        );
        return None;
    }
    if args.iter().any(|arg| arg.is_none()) {
        tcx.sess.span_err(
            attr.span,
            "Attribute `kani::stub` takes two path arguments; \
                found argument that is not a path",
        );
        return None;
    }

    // Resolve the attribute arguments to `DefId`s
    let current_module = tcx.parent_module_from_def_id(harness);
    let resolve = |name: &str| -> Option<DefId> {
        let maybe_resolved = resolve_path(tcx, current_module, name);
        if let Some(def_id) = maybe_resolved {
            tracing::debug!(?def_id, "Resolved {name} to {}", tcx.def_path_str(def_id));
        } else {
            tcx.sess.span_err(attr.span, format!("unable to resolve function/method: {name}"));
        }
        maybe_resolved
    };
    let orig = resolve(args[0].as_deref().unwrap());
    let stub = resolve(args[1].as_deref().unwrap());
    Some((orig?, stub?))
}

/// Updates the running map `stub_pairs` that maps a function/method to its
/// stub. Errors if a function/method is mapped more than once.
fn update_stub_mapping(
    tcx: TyCtxt,
    harness: LocalDefId,
    attr: &Attribute,
    stub_pairs: &mut FxHashMap<DefPathHash, DefPathHash>,
) {
    if let Some((orig_id, stub_id)) = extract_stubbing_pair(tcx, harness, attr) {
        let orig_hash = tcx.def_path_hash(orig_id);
        let stub_hash = tcx.def_path_hash(stub_id);
        let other_opt = stub_pairs.insert(orig_hash, stub_hash);
        if let Some(other) = other_opt {
            if other != stub_hash {
                tcx.sess.span_err(
                    attr.span,
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
