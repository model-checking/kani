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
use rustc_ast::{ItemKind, UseTreeKind};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{Callbacks, Compilation, run_compiler};
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
use rustc_public::rustc_internal;
use rustc_session::config::ErrorOutputType;
use rustc_session::parse::ParseSess;
use rustc_span::edition::Edition;
use rustc_span::source_map::{FileLoader, RealFileLoader};
use rustc_span::{FileName, RealFileName};
use std::io;
use std::path::Path;
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

struct KaniFileLoader;

impl FileLoader for KaniFileLoader {
    fn file_exists(&self, path: &Path) -> bool {
        RealFileLoader.file_exists(path)
    }

    fn read_file(&self, path: &Path) -> io::Result<String> {
        struct AstNoAnn;
        impl rustc_ast_pretty::pprust::PpAnn for AstNoAnn {}

        if path.ends_with("library/std/src/macros.rs") {
            let psess = ParseSess::new(vec![]);
            let filename = FileName::Real(RealFileName::LocalPath(path.to_owned()));

            let mut module = rustc_parse::new_parser_from_source_str(
                &psess,
                filename.clone(),
                RealFileLoader.read_file(path)?,
            )
            .unwrap()
            .parse_crate_mod()
            .unwrap();

            // Remove definitions of macros we will overwrite.
            module.items.retain(|item| match item.kind {
                ItemKind::MacroDef(name, _) => {
                    !matches!(name.as_str(), "print" | "eprint" | "println" | "eprintln" | "panic")
                }
                _ => true,
            });

            // Parse the macro definition overwrites and append them to the current std::macros module.
            let macros_mod = rustc_parse::new_parser_from_source_str(
                &psess,
                FileName::Custom("kani-macros".to_owned()),
                include_str!("../../library/std/src/macros.rs").to_owned(),
            )
            .unwrap()
            .parse_crate_mod()
            .unwrap()
            .items;
            module.items.extend(macros_mod);

            Ok(rustc_ast_pretty::pprust::print_crate(
                psess.source_map(),
                &module,
                filename,
                "".to_owned(),
                &AstNoAnn,
                false,
                Edition::Edition2024,
                &psess.attr_id_generator,
            ))
        } else {
            RealFileLoader.read_file(path)
        }
    }

    fn read_binary_file(&self, path: &Path) -> io::Result<Arc<[u8]>> {
        RealFileLoader.read_binary_file(path)
    }
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
        // `kani-driver` passes the `kani-compiler` specific arguments through llvm-args, so extract them here.
        args.extend(config.opts.cg.llvm_args.iter().cloned());
        let args = Arguments::parse_from(args);
        init_session(&args, matches!(config.opts.error_format, ErrorOutputType::Json { .. }));

        // Configure queries.
        let queries = &mut (*self.queries.lock().unwrap());
        queries.set_args(args);
        debug!(?queries, "config end");

        config.file_loader = Some(Box::new(KaniFileLoader));
    }

    fn after_crate_root_parsing(
        &mut self,
        compiler: &rustc_interface::interface::Compiler,
        krate: &mut rustc_ast::Crate,
    ) -> Compilation {
        if compiler.sess.opts.crate_name.as_deref() == Some("std") {
            for item in &mut krate.items {
                if let ItemKind::Use(use_) = &mut item.kind
                    && let [root] = &*use_.prefix.segments
                    && root.ident.as_str() == "core"
                    && let UseTreeKind::Nested { items, .. } = &mut use_.kind
                {
                    // Remove all re-exports of core macros that we overwrite to prevent conflicts.
                    items.retain(|item| {
                        !matches!(
                            item.0.ident().as_str(),
                            "assert"
                                | "assert_eq"
                                | "assert_ne"
                                | "debug_assert"
                                | "debug_assert_eq"
                                | "debug_assert_ne"
                                | "unreachable"
                        )
                    });
                }
            }
        }

        Compilation::Continue
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
