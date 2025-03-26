// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines all compiler extensions that form the Kani compiler.
//!
//! The [KaniCompiler] can be used across multiple rustc driver runs ([`rustc_driver::run_compiler`]),
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

use crate::args::{Arguments, BackendOption};
#[cfg(feature = "llbc")]
use crate::codegen_aeneas_llbc::LlbcCodegenBackend;
#[cfg(feature = "cprover")]
use crate::codegen_cprover_gotoc::GotocCodegenBackend;
use crate::kani_middle::check_crate_items;
use crate::kani_queries::QueryDb;
use crate::session::init_session;
use clap::Parser;
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{Callbacks, Compilation, run_compiler};
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::ErrorOutputType;
use rustc_smir::rustc_internal;
use std::sync::{Arc, Mutex};
use tracing::debug;

/// Run the Kani flavour of the compiler.
/// This may require multiple runs of the rustc driver ([`rustc_driver::run_compiler`]).
pub fn run(args: Vec<String>) {
    let mut kani_compiler = KaniCompiler::new();
    kani_compiler.run(args);
}

/// Configure the LLBC backend (Aeneas's IR).
#[allow(unused)]
fn llbc_backend(_queries: Arc<Mutex<QueryDb>>) -> Box<dyn CodegenBackend> {
    #[cfg(feature = "llbc")]
    return Box::new(LlbcCodegenBackend::new(_queries));
    #[cfg(not(feature = "llbc"))]
    unreachable!()
}

/// Configure the cprover backend that generates goto-programs.
#[allow(unused)]
fn cprover_backend(_queries: Arc<Mutex<QueryDb>>) -> Box<dyn CodegenBackend> {
    #[cfg(feature = "cprover")]
    return Box::new(GotocCodegenBackend::new(_queries));
    #[cfg(not(feature = "cprover"))]
    unreachable!()
}

#[cfg(any(feature = "cprover", feature = "llbc"))]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<dyn CodegenBackend> {
    let backend = queries.lock().unwrap().args().backend;
    match backend {
        #[cfg(feature = "cprover")]
        BackendOption::CProver => cprover_backend(queries),
        #[cfg(feature = "llbc")]
        BackendOption::Llbc => llbc_backend(queries),
    }
}

/// Fallback backend. It will trigger an error if no backend has been enabled.
#[cfg(not(any(feature = "cprover", feature = "llbc")))]
fn backend(queries: Arc<Mutex<QueryDb>>) -> Box<CodegenBackend> {
    compile_error!("No backend is available. Use `cprover` or `llbc`.");
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
    pub fn run(&mut self, args: Vec<String>) {
        debug!(?args, "run_compilation_session");
        run_compiler(&args, self);
    }
}

/// Use default function implementations.
impl Callbacks for KaniCompiler {
    /// Configure the [KaniCompiler] `self` object during the [CompilationStage::Init].
    fn config(&mut self, config: &mut Config) {
        let mut args = vec!["kani-compiler".to_string()];
        config.make_codegen_backend = Some(Box::new({
            let queries = self.queries.clone();
            move |_cfg| backend(queries)
        }));
        args.extend(config.opts.cg.llvm_args.iter().cloned());
        let args = Arguments::parse_from(args);
        init_session(&args, matches!(config.opts.error_format, ErrorOutputType::Json { .. }));

        // Configure queries.
        let queries = &mut (*self.queries.lock().unwrap());
        queries.set_args(args);
        debug!(?queries, "config end");
    }

    /// After analysis, we check the crate items for Kani API misuse or configuration issues.
    fn after_analysis(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: TyCtxt<'_>,
    ) -> Compilation {
        rustc_internal::run(tcx, || {
            check_crate_items(tcx, self.queries.lock().unwrap().args().ignore_global_asm);
        })
        .unwrap();
        Compilation::Continue
    }
}
