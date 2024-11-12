// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is the main entry point to our compiler driver. This code accepts a few options that
//! can be used to configure goto-c compilation as well as all other flags supported by rustc.
//!
//! Like miri, clippy, and other tools developed on the top of rustc, we rely on the
//! rustc_private feature and a specific version of rustc.
#![feature(extern_types)]
#![recursion_limit = "256"]
#![feature(box_patterns)]
#![feature(rustc_private)]
#![feature(more_qualified_paths)]
#![feature(iter_intersperse)]
#![feature(let_chains)]
#![feature(f128)]
#![feature(f16)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(float_next_up_down)]
extern crate rustc_abi;
extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_codegen_ssa;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_interface;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_session;
extern crate rustc_smir;
extern crate rustc_span;
extern crate rustc_target;
extern crate stable_mir;
// We can't add this directly as a dependency because we need the version to match rustc
extern crate tempfile;

mod args;
#[cfg(feature = "llbc")]
mod codegen_aeneas_llbc;
#[cfg(feature = "cprover")]
mod codegen_cprover_gotoc;
mod intrinsics;
mod kani_compiler;
mod kani_middle;
mod kani_queries;
mod session;

use rustc_driver::{RunCompiler, TimePassesCallbacks};
use std::env;
use std::process::ExitCode;

/// Main function. Configure arguments and run the compiler.
fn main() -> ExitCode {
    session::init_panic_hook();
    let (kani_compiler, rustc_args) = is_kani_compiler(env::args().collect());

    // Configure and run compiler.
    if kani_compiler {
        kani_compiler::run(rustc_args)
    } else {
        let mut callbacks = TimePassesCallbacks::default();
        let compiler = RunCompiler::new(&rustc_args, &mut callbacks);
        if compiler.run().is_err() { ExitCode::FAILURE } else { ExitCode::SUCCESS }
    }
}

/// Return whether we should run our flavour of the compiler, and which arguments to pass to rustc.
///
/// We add a `--kani-compiler` argument to run the Kani version of the compiler, which needs to be
/// filtered out before passing the arguments to rustc.
///
/// All other Kani arguments are today located inside `--llvm-args`.
pub fn is_kani_compiler(args: Vec<String>) -> (bool, Vec<String>) {
    assert!(!args.is_empty(), "Arguments should always include executable name");
    const KANI_COMPILER: &str = "--kani-compiler";
    let mut has_kani_compiler = false;
    let new_args = args
        .into_iter()
        .filter(|arg| {
            if arg == KANI_COMPILER {
                has_kani_compiler = true;
                false
            } else {
                true
            }
        })
        .collect();
    (has_kani_compiler, new_args)
}
