// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This file contains code for extracting stubbing-related attributes.

use rustc_ast::Attribute;
use rustc_data_structures::fx::FxHashMap;
use rustc_driver::RunCompiler;
use rustc_driver::{Callbacks, Compilation};
use rustc_errors::ErrorGuaranteed;
use rustc_interface::interface::Compiler;
use rustc_interface::Queries;
use rustc_middle::ty::TyCtxt;

use crate::kani_middle::attributes::{extract_path_arguments, partition_kanitool_attributes};

/// Collects the stubs from the harnesses in a crate, running rustc with the
/// supplied arguments `rustc_args`.
pub fn collect_stub_mappings(
    rustc_args: &[String],
) -> Result<FxHashMap<String, FxHashMap<String, String>>, ErrorGuaranteed> {
    let mut callbacks = CollectorCallbacks::default();
    let compiler = RunCompiler::new(rustc_args, &mut callbacks);
    compiler.run().and_then(|_| Ok(callbacks.stub_mapping))
}

/// A rustc callback that is used to collect the stub mappings specified for
/// each harness.
#[derive(Default)]
struct CollectorCallbacks {
    stub_mapping: FxHashMap<String, FxHashMap<String, String>>,
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
                let def_id = item.owner_id.def_id.to_def_id();
                let (proof, other) = partition_kanitool_attributes(tcx.get_attrs_unchecked(def_id));
                // Ignore anything that is not a harness
                if proof.is_empty() {
                    continue;
                }
                let mut stub_pairs = FxHashMap::default();
                for (name, attr) in other {
                    if name == "stub" {
                        Self::update_stub_mapping(tcx, attr, &mut stub_pairs);
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

impl CollectorCallbacks {
    /// Given a `kani::stub` attribute, tries to extract a pair of paths (the
    /// original function/method, and its stub). Returns `None` and errors if
    /// the attribute's arguments are not two paths.
    fn extract_stubbing_pair(tcx: TyCtxt, attr: &Attribute) -> Option<(String, String)> {
        if let Some(args) = extract_path_arguments(attr) {
            if args.len() == 2 {
                // TODO: We need to do actual path resolution, instead of just
                // taking these names verbatim.
                // <https://github.com/model-checking/kani/issues/1866>
                return Some((args[0].clone(), args[1].clone()));
            } else {
                tcx.sess.span_err(
                    attr.span,
                    format!(
                        "Attribute `kani::stub` takes two path arguments; found {}",
                        args.len()
                    ),
                );
            }
        } else {
            tcx.sess.span_err(
                attr.span,
                "Attribute `kani::stub` takes two path arguments; \
                found argument that is not a path",
            );
        }
        None
    }

    /// Updates the running map `stub_pairs` that maps a function/method to its
    /// stub. Errors if a function/method is mapped more than once.
    fn update_stub_mapping(
        tcx: TyCtxt,
        attr: &Attribute,
        stub_pairs: &mut FxHashMap<String, String>,
    ) {
        if let Some((original, replacement)) = Self::extract_stubbing_pair(tcx, attr) {
            let other = stub_pairs.insert(original.clone(), replacement.clone());
            if let Some(other) = other {
                tcx.sess.span_err(
                    attr.span,
                    format!(
                        "duplicate stub mapping: {} mapped to {} and {}",
                        original, replacement, other
                    ),
                );
            }
        }
    }
}
