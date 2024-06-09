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

use crate::args::Arguments;
#[cfg(feature = "cprover")]
use crate::codegen_cprover_gotoc::GotocCodegenBackend;
use crate::kani_middle::check_crate_items;
use crate::kani_queries::QueryDb;
use crate::session::init_session;
use clap::Parser;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::Config;
use rustc_session::config::ErrorOutputType;
use rustc_smir::rustc_internal;
use rustc_span::ErrorGuaranteed;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use tracing::debug;

/// Run the Kani flavour of the compiler.
/// This may require multiple runs of the rustc driver ([RunCompiler::run]).
pub fn run(args: Vec<String>) -> ExitCode {
    let mut kani_compiler = KaniCompiler::new();
    match kani_compiler.run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
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
/// state machine.
struct KaniCompiler {
    /// Store the query database. The queries should be initialized as part of `config` when the
    /// compiler state is Init.
    /// Note that we need to share the queries with the backend before `config` is called.
    pub queries: Arc<Mutex<QueryDb>>,
}

impl KaniCompiler {
    /// Create a new [KaniCompiler] instance.
    pub fn new() -> KaniCompiler {
        KaniCompiler { queries: QueryDb::new() }
    }

    /// Compile the current crate with the given arguments.
    ///
    /// Since harnesses may have different attributes that affect compilation, Kani compiler can
    /// actually invoke the rust compiler multiple times.
    pub fn run(&mut self, args: Vec<String>) -> Result<(), ErrorGuaranteed> {
        debug!(?args, "run_compilation_session");
        let queries = self.queries.clone();
        let mut compiler = RunCompiler::new(&args, self);
        compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| backend(queries))));
        compiler.run()?;
        Ok(())
    }
}

/// Use default function implementations.
impl Callbacks for KaniCompiler {
    /// Configure the [KaniCompiler] `self` object during the [CompilationStage::Init].
    fn config(&mut self, config: &mut Config) {
        let mut args = vec!["kani-compiler".to_string()];
        args.extend(config.opts.cg.llvm_args.iter().cloned());
        let args = Arguments::parse_from(args);
        init_session(&args, matches!(config.opts.error_format, ErrorOutputType::Json { .. }));

        // Configure queries.
        let queries = &mut (*self.queries.lock().unwrap());
        queries.set_args(args);
        debug!(?queries, "config end");
    }

    /// After analysis, we check the crate items for Kani API misusage or configuration issues.
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        rustc_queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        rustc_queries.global_ctxt().unwrap().enter(|tcx| {
            rustc_internal::run(tcx, || {
                check_crate_items(tcx, self.queries.lock().unwrap().args().ignore_global_asm);
            })
            .unwrap()
        });
        Compilation::Continue
    }
}
