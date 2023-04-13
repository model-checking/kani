// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is the main entry point to our compiler driver. This code accepts a few options that
//! can be used to configure goto-c compilation as well as all other flags supported by rustc.
//!
//! Like miri, clippy, and other tools developed on the top of rustc, we rely on the
//! rustc_private feature and a specific version of rustc.
#![deny(warnings)]
#![feature(extern_types)]
#![recursion_limit = "256"]
#![feature(box_patterns)]
#![feature(once_cell)]
#![feature(rustc_private)]
#![feature(more_qualified_paths)]
#![feature(iter_intersperse)]
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

use rustc_driver::{RunCompiler, TimePassesCallbacks};
use std::env;
use std::process::ExitCode;

/// Main function. Configure arguments and run the compiler.
fn main() -> ExitCode {
    session::init_panic_hook();
    let (kani_compiler, rustc_args) = parser::is_kani_compiler(env::args().collect());

    // Configure and run compiler.
    if kani_compiler {
        kani_compiler::run(rustc_args)
    } else {
        let mut callbacks = TimePassesCallbacks::default();
        let compiler = RunCompiler::new(&rustc_args, &mut callbacks);
        if compiler.run().is_err() { ExitCode::FAILURE } else { ExitCode::SUCCESS }
    }
}
