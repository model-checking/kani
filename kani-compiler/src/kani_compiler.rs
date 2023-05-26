// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines all compiler extensions that form the Kani compiler.
//!
//! The [KaniCompiler] can be used across multiple rustc driver runs ([RunCompiler::run()]),
//! which is used to implement stubs.
//!
//! In the first run, [KaniCompiler::config] will implement the compiler configuration and it will
//! also collect any stubs that may need to be applied. This method will be a no-op for any
//! subsequent runs. The [KaniCompiler] will parse options that are passed via `-C llvm-args`.
//!
//! If no stubs need to be applied, the compiler will proceed to generate goto code, and it won't
//! need any extra runs. However, if stubs are required, we will have to restart the rustc driver
//! in order to apply the stubs. For the subsequent runs, we add the stub configuration to
//! `-C llvm-args`.

#[cfg(feature = "cprover")]
use crate::codegen_cprover_gotoc::GotocCodegenBackend;
use crate::kani_middle::stubbing;
use crate::parser::{self, KaniCompilerParser};
use crate::session::init_session;
use clap::ArgMatches;
use itertools::Itertools;
use kani_queries::{QueryDb, ReachabilityType, UserInput};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_hir::definitions::DefPathHash;
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::ErrorOutputType;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use tracing::debug;

/// Run the Kani flavour of the compiler.
/// This may require multiple runs of the rustc driver ([RunCompiler::run]).
pub fn run(mut args: Vec<String>) -> ExitCode {
    let mut kani_compiler = KaniCompiler::new();
    while !args.is_empty() {
        let queries = kani_compiler.queries.clone();
        let mut compiler = RunCompiler::new(&args, &mut kani_compiler);
        compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| backend(queries))));
        if compiler.run().is_err() {
            return ExitCode::FAILURE;
        }

        args = kani_compiler.post_process(args).unwrap_or_default();
        debug!("Finish driver run. {}", if args.is_empty() { "Done" } else { "Run again" });
    }
    ExitCode::SUCCESS
}

/// Configure the cprover backend that generate goto-programs.
#[cfg(feature = "cprover")]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<dyn CodegenBackend> {
    Box::new(GotocCodegenBackend::new(queries))
}

/// Fallback backend. It will trigger an error if no backend has been enabled.
#[cfg(not(feature = "cprover"))]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<CodegenBackend> {
    compile_error!("No backend is available. Only supported value today is `cprover`");
}

/// This object controls the compiler behavior.
///
/// It is responsible for initializing the query database, as well as controlling the compiler
/// state machine. For stubbing, we may require multiple iterations of the rustc driver, which is
/// controlled and configured via KaniCompiler.
struct KaniCompiler {
    /// Store the queries database. The queries should be initialized as part of `config`.
    pub queries: Arc<Mutex<QueryDb>>,
    /// Store the stubs that shall be applied if any.
    stubs: Option<FxHashMap<DefPathHash, DefPathHash>>,
    /// Store the arguments for kani compiler.
    args: Option<ArgMatches>,
}

impl KaniCompiler {
    /// Create a new [KaniCompiler] instance.
    pub fn new() -> KaniCompiler {
        KaniCompiler { queries: QueryDb::new(), stubs: None, args: None }
    }

    /// Method to be invoked after a rustc driver run.
    /// It will return a list of arguments that should be used in a subsequent call to rustc
    /// driver. It will return None if it has finished compiling everything.
    pub fn post_process(&mut self, old_args: Vec<String>) -> Option<Vec<String>> {
        let stubs = self.stubs.replace(FxHashMap::default()).unwrap_or_default();
        if stubs.is_empty() {
            None
        } else {
            let mut new_args = old_args;
            new_args.push(stubbing::mk_rustc_arg(&stubs));
            Some(new_args)
        }
    }

    /// Collect the stubs that shall be applied in the next run.
    fn collect_stubs(&self, tcx: TyCtxt) -> FxHashMap<DefPathHash, DefPathHash> {
        let all_stubs = stubbing::collect_stub_mappings(tcx);
        if all_stubs.is_empty() {
            FxHashMap::default()
        } else if let Some(harnesses) =
            self.args.as_ref().unwrap().get_many::<String>(parser::HARNESS)
        {
            let mappings = filter_stub_mapping(harnesses.collect(), all_stubs);
            if mappings.len() > 1 {
                tcx.sess.err(format!(
                    "Failed to apply stubs. Harnesses with stubs must be verified separately. Found: `{}`",
                     mappings.into_keys().join("`, `")));
                FxHashMap::default()
            } else {
                mappings.into_values().next().unwrap_or_default()
            }
        } else {
            // No harness was provided. Nothing to do.
            FxHashMap::default()
        }
    }
}

/// Use default function implementations.
impl Callbacks for KaniCompiler {
    fn config(&mut self, config: &mut Config) {
        if self.args.is_none() {
            let mut args = vec!["kani-compiler".to_string()];
            args.extend(config.opts.cg.llvm_args.iter().cloned());
            let matches = parser::parser().get_matches_from(&args);
            init_session(
                &matches,
                matches!(config.opts.error_format, ErrorOutputType::Json { .. }),
            );

            // Configure queries.
            let queries = &mut (*self.queries.lock().unwrap());
            queries.set_emit_vtable_restrictions(matches.get_flag(parser::RESTRICT_FN_PTRS));
            queries
                .set_check_assertion_reachability(matches.get_flag(parser::ASSERTION_REACH_CHECKS));
            queries.set_output_pretty_json(matches.get_flag(parser::PRETTY_OUTPUT_FILES));
            queries.set_ignore_global_asm(matches.get_flag(parser::IGNORE_GLOBAL_ASM));
            queries.set_write_json_symtab(
                cfg!(feature = "write_json_symtab") || matches.get_flag(parser::WRITE_JSON_SYMTAB),
            );
            queries.set_reachability_analysis(matches.reachability_type());
            queries.set_enforce_contracts(matches.get_flag(parser::ENFORCE_CONTRACTS));
            queries.set_replace_with_contracts(matches.get_flag(parser::REPLACE_WITH_CONTRACTS));

            if let Some(features) = matches.get_many::<String>(parser::UNSTABLE_FEATURE) {
                queries.set_unstable_features(&features.cloned().collect::<Vec<_>>());
            }

            // If appropriate, collect and set the stub mapping.
            if matches.get_flag(parser::ENABLE_STUBBING)
                && queries.get_reachability_analysis() == ReachabilityType::Harnesses
            {
                queries.set_stubbing_enabled(true);
            }
            self.args = Some(matches);
            debug!(?queries, "config end");
        }
    }

    /// Collect stubs and return whether we should restart rustc's driver or not.
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        rustc_queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        if self.stubs.is_none() && self.queries.lock().unwrap().get_stubbing_enabled() {
            rustc_queries.global_ctxt().unwrap().enter(|tcx| {
                let stubs = self.stubs.insert(self.collect_stubs(tcx));
                debug!(?stubs, "after_analysis");
                if stubs.is_empty() { Compilation::Continue } else { Compilation::Stop }
            })
        } else {
            // There is no need to initialize stubs, keep compiling.
            Compilation::Continue
        }
    }
}

/// Find the stub mapping for the given harnesses.
///
/// This function is necessary because Kani currently allows a harness to be
/// specified as a filter, whereas stub mappings use fully qualified names.
fn filter_stub_mapping(
    harnesses: FxHashSet<&String>,
    mut stub_mappings: FxHashMap<String, FxHashMap<DefPathHash, DefPathHash>>,
) -> FxHashMap<String, FxHashMap<DefPathHash, DefPathHash>> {
    stub_mappings.retain(|name, _| {
        harnesses.contains(name) || harnesses.iter().any(|harness| name.contains(*harness))
    });
    stub_mappings
}
