// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines all compiler extensions that form the Kani compiler.

use crate::codegen_cprover_gotoc::GotocCodegenBackend;
use crate::kani_middle::stubbing;
use crate::parser::{self, KaniCompilerParser};
use crate::session::init_session;
use clap::ArgMatches;
use kani_queries::{QueryDb, ReachabilityType, UserInput};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_data_structures::fx::FxHashMap;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_hir::definitions::DefPathHash;
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::ErrorOutputType;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

pub fn run(mut args: Vec<String>) -> ExitCode {
    let mut kani_compiler = KaniCompiler::new();
    while !args.is_empty() {
        let queries = kani_compiler.queries.clone();
        let mut compiler = RunCompiler::new(&args, &mut kani_compiler);
        compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| backend(queries))));
        if compiler.run().is_err() {
            return ExitCode::FAILURE;
        }

        args = kani_compiler.post_process(args);
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "cprover")]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<dyn CodegenBackend> {
    Box::new(GotocCodegenBackend::new(queries))
}

#[cfg(not(feature = "cprover"))]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<CodegenBackend> {
    compile_error!("No backend is available. Only supported value today is `cprover`");
}

/// Empty struct since we don't support any callbacks yet.
struct KaniCompiler {
    /// Store the queries database. The queries should be initialized as part of `config`.
    pub queries: Arc<Mutex<QueryDb>>,
    /// Store the stubs that shall be applied if any.
    stubs: Option<FxHashMap<DefPathHash, DefPathHash>>,
    /// Store the arguments for kani compiler.
    args: Option<ArgMatches>,
}

impl KaniCompiler {
    pub fn new() -> KaniCompiler {
        KaniCompiler { queries: QueryDb::new(), stubs: None, args: None }
    }

    pub fn post_process(&mut self, old_args: Vec<String>) -> Vec<String> {
        let stubs = self.stubs.take().unwrap_or_default();
        if stubs.is_empty() {
            vec![]
        } else {
            let mut new_args = old_args;
            new_args.push(stubbing::mk_rustc_arg(&stubs));
            new_args
        }
    }

    fn collect_stubs(&self, tcx: TyCtxt) -> FxHashMap<DefPathHash, DefPathHash> {
        let all_stubs = stubbing::collect_stub_mappings(tcx);
        if all_stubs.is_empty() {
            FxHashMap::default()
        } else if let Some(harness) = self.args.as_ref().unwrap().get_one::<String>(parser::HARNESS)
        {
            find_harness_stub_mapping(harness, all_stubs).unwrap_or_default()
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
            let matches = parser::parser().get_matches_from(&config.opts.cg.llvm_args);
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
            queries.set_reachability_analysis(matches.reachability_type());
            #[cfg(feature = "unsound_experiments")]
            crate::unsound_experiments::arg_parser::add_unsound_experiment_args_to_queries(
                &mut queries,
                &matches,
            );

            // If appropriate, collect and set the stub mapping.
            if matches.get_flag(parser::ENABLE_STUBBING)
                && queries.get_reachability_analysis() == ReachabilityType::Harnesses
            {
                queries.set_stubbing_enabled(true);
            }
            self.args = Some(matches);
        }
    }

    // TODO: What if we try after_expansion??
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        rustc_queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        if self.stubs.is_none() && self.queries.lock().unwrap().get_stubbing_enabled() {
            rustc_queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
                let stubs = self.stubs.insert(self.collect_stubs(tcx));
                if stubs.is_empty() { Compilation::Continue } else { Compilation::Stop }
            })
        } else {
            // There is no need to initialize stubs, keep compiling.
            Compilation::Continue
        }
    }
}

/// Find the stub mapping for the given harness.
///
/// This function is necessary because Kani currently allows a harness to be
/// specified by a partially qualified name, whereas stub mappings use fully
/// qualified names.
fn find_harness_stub_mapping(
    harness: &str,
    stub_mappings: FxHashMap<String, FxHashMap<DefPathHash, DefPathHash>>,
) -> Option<FxHashMap<DefPathHash, DefPathHash>> {
    let suffix = String::from("::") + harness;
    for (name, mapping) in stub_mappings {
        if name == harness || name.ends_with(&suffix) {
            return Some(mapping);
        }
    }
    None
}
