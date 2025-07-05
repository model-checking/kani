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
#![feature(f128)]
#![feature(f16)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(cfg_version)]
// Once the `stable` branch is at 1.86 or later, remove this line, since float_next_up_down is stabilized
#![cfg_attr(not(version("1.86")), feature(float_next_up_down))]
#![feature(try_blocks)]
extern crate rustc_abi;
extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_codegen_ssa;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_pretty;
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

use rustc_driver::{TimePassesCallbacks, run_compiler};
use std::env;

/// Main function. Configure arguments and run the compiler.
fn main() {
    session::init_panic_hook();
    let (kani_compiler, rustc_args) = is_kani_compiler(env::args().collect());

    // Configure and run compiler.
    if kani_compiler {
        kani_compiler::run(rustc_args);
    } else {
        let mut callbacks = TimePassesCallbacks::default();
        run_compiler(&rustc_args, &mut callbacks);
    }
}

/// Return whether we should run our flavour of the compiler, and which arguments to pass to rustc.
///
/// `kani-driver` adds a `--kani-compiler` argument to run the Kani version of the compiler, which needs to be
/// filtered out before passing the arguments to rustc.
/// All other Kani arguments are today located inside `--llvm-args`.
///
/// This function returns `true` for rustc invocations that originate from our rustc / cargo rustc invocations in `kani-driver`.
/// It returns `false` for rustc invocations that cargo adds in the process of executing the `kani-driver` rustc command.
/// For example, if we are compiling a crate that has a build.rs file, cargo will compile and run that build script
/// (c.f. https://doc.rust-lang.org/cargo/reference/build-scripts.html#life-cycle-of-a-build-script).
/// The build script should be compiled with normal rustc, not the Kani compiler.
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
