// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is the main entry point to our compiler driver. This code accepts a few options that
//! can be used to configure goto-c compilation as well as all other flags supported by rustc.
//!
//! Like miri, clippy, and other tools developed on the top of rustc, we rely on the
//! rustc_private feature and a specific version of rustc.
//#![deny(warnings)]
#![feature(extern_types)]
#![recursion_limit = "256"]
#![feature(box_patterns)]
#![feature(once_cell)]
#![feature(rustc_private)]
#![feature(more_qualified_paths)]
extern crate rustc_ast;
extern crate rustc_codegen_ssa;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_interface;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
// We can't add this directly as a dependency because we need the version to match rustc
extern crate tempfile;

#[cfg(feature = "cprover")]
mod codegen_cprover_gotoc;
mod kani_compiler;
mod kani_middle;
mod parser;
mod session;
mod unsound_experiments;

use std::env;

use kani_compiler::KaniCallbacks;
use kani_queries::QueryDb;
use rustc_driver::RunCompiler;

/// Main function. Configure arguments and run the compiler.
fn main() -> Result<(), &'static str> {
    session::init_panic_hook();
    let (backend, rustc_args) = parser::extract_backend_flag(env::args().collect());

    // Configure and run compiler.
    let queries = QueryDb::new();
    let mut callbacks = KaniCallbacks { queries: queries.clone() };
    let mut compiler = RunCompiler::new(&rustc_args, &mut callbacks);
    if !backend.is_empty() {
        assert_eq!(backend, "goto-c", "Unsupported backend selected");
        if cfg!(feature = "cprover") {
            compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| {
                Box::new(codegen_cprover_gotoc::GotocCodegenBackend::new(queries))
            })));
        } else {
            return Err("Kani was configured without 'cprover' feature. You must enable this \
            feature in order to use --goto-c argument.");
        }
    }
    compiler.run().or(Err("Failed to compile crate."))
}
