// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is the main entry point to our compiler driver. This code accepts a few options that
//! can be used to configure goto-c compilation as well as all other flags supported by rustc.
//!
//! Like miri, clippy, and other tools developed on the top of rustc, we rely on the
//! rustc_private feature and a specific version of rustc.
#![deny(warnings)]
#![feature(bool_to_option)]
#![feature(crate_visibility_modifier)]
#![feature(extern_types)]
#![feature(nll)]
#![recursion_limit = "256"]
#![feature(box_patterns)]
#![feature(once_cell)]
#![feature(rustc_private)]
extern crate rustc_ast;
extern crate rustc_codegen_ssa;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

#[cfg(feature = "cprover")]
mod codegen_cprover_gotoc;
mod parser;
mod session;

use crate::session::init_session;
use clap::ArgMatches;
use kani_queries::{QueryDb, UserInput};
use rustc_driver::{Callbacks, RunCompiler};
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::rc::Rc;

/// This function generates all rustc configurations required by our goto-c codegen.
fn rustc_gotoc_flags(lib_path: &str) -> Vec<String> {
    // The option below provides a mechanism by which definitions in the
    // standard library can be overriden. See
    // https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/.E2.9C.94.20Globally.20override.20an.20std.20macro/near/268873354
    // for more details.
    let kani_std_rlib = PathBuf::from(lib_path).join("libstd.rlib");
    let kani_std_wrapper = format!("noprelude:std={}", kani_std_rlib.to_str().unwrap());
    let args = vec![
        "-C",
        "overflow-checks=on",
        "-C",
        "panic=abort",
        "-Z",
        "unstable-options",
        "-Z",
        "panic_abort_tests=yes",
        "-Z",
        "trim-diagnostic-paths=no",
        "-Z",
        "human_readable_cgu_names",
        "--cfg=kani",
        "-Z",
        "crate-attr=feature(register_tool)",
        "-Z",
        "crate-attr=register_tool(kanitool)",
        "-L",
        lib_path,
        "--extern",
        "kani",
        "--extern",
        kani_std_wrapper.as_str(),
    ];
    args.iter().map(|s| s.to_string()).collect()
}

/// Main function. Configure arguments and run the compiler.
fn main() -> Result<(), &'static str> {
    let args = parser::command_arguments(&env::args().collect());
    let matches = parser::parser().get_matches_from(args);
    init_session(&matches);

    // Configure queries.
    let mut queries = QueryDb::default();
    if let Some(symbol_table_passes) = matches.values_of_os(parser::SYM_TABLE_PASSES) {
        queries.set_symbol_table_passes(symbol_table_passes.map(convert_arg).collect::<Vec<_>>());
    }
    queries.set_emit_vtable_restrictions(matches.is_present(parser::RESTRICT_FN_PTRS));
    queries.set_check_assertion_reachability(matches.is_present(parser::ASSERTION_REACH_CHECKS));
    queries.set_output_pretty_json(matches.is_present(parser::PRETTY_OUTPUT_FILES));
    queries.set_ignore_global_asm(matches.is_present(parser::IGNORE_GLOBAL_ASM));

    // Generate rustc args.
    let rustc_args = generate_rustc_args(&matches);

    // Configure and run compiler.
    let mut callbacks = KaniCallbacks {};
    let mut compiler = RunCompiler::new(&rustc_args, &mut callbacks);
    if matches.is_present("goto-c") {
        if cfg!(feature = "cprover") {
            compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| {
                codegen_cprover_gotoc::GotocCodegenBackend::new(&Rc::new(queries))
            })));
        } else {
            return Err("Kani was configured without 'cprover' feature. You must enable this \
            feature in order to use --goto-c argument.");
        }
    }
    compiler.run().or(Err("Failed to compile crate."))
}

/// Empty struct since we don't support any callbacks yet.
struct KaniCallbacks {}

/// Use default function implementations.
impl Callbacks for KaniCallbacks {}

/// Generate the arguments to pass to rustc_driver.
fn generate_rustc_args(args: &ArgMatches) -> Vec<String> {
    let mut gotoc_args =
        rustc_gotoc_flags(&args.value_of(parser::KANI_LIB).unwrap_or(std::env!("KANI_LIB_PATH")));
    let mut rustc_args = vec![String::from("rustc")];
    if args.is_present(parser::GOTO_C) {
        rustc_args.append(&mut gotoc_args);
    }

    if args.is_present(parser::RUSTC_VERSION) {
        rustc_args.push(String::from("--version"))
    }

    if args.is_present(parser::JSON_OUTPUT) {
        rustc_args.push(String::from("--error-format=json"));
    }

    if let Some(extra_flags) = args.values_of_os(parser::RUSTC_OPTIONS) {
        extra_flags.for_each(|arg| rustc_args.push(convert_arg(arg)));
    }
    let sysroot = sysroot_path(args.value_of(parser::SYSROOT)).expect(
        "[Error] Invalid sysroot. Rebuild Kani or provide the path to rust sysroot using --sysroot option",
    );
    rustc_args.push(String::from("--sysroot"));
    rustc_args.push(convert_arg(sysroot.as_os_str()));
    tracing::debug!(?rustc_args, "Compile");
    rustc_args
}

/// Try to generate the rustup toolchain path.
fn toolchain_path(home: Option<String>, toolchain: Option<String>) -> Option<PathBuf> {
    match (home, toolchain) {
        (Some(home), Some(toolchain)) => {
            Some([home, String::from("toolchains"), toolchain].iter().collect::<PathBuf>())
        }
        _ => None,
    }
}

/// Convert an argument from OsStr to String.
/// If conversion fails, panic with a custom message.
fn convert_arg(arg: &OsStr) -> String {
    arg.to_str()
        .expect(format!("[Error] Cannot parse argument \"{:?}\".", arg).as_str())
        .to_string()
}

/// Get the sysroot, following the order bellow:
/// - "--sysroot" command line argument
/// - compile-time environment
///    - $SYSROOT
///    - $RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN
///
/// We currently don't support:
/// - runtime environment
///    - $SYSROOT
///    - $RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN
/// - rustc --sysroot
///
/// since we rely on specific nightly version of rustc which may not be compatible with the workspace rustc.
fn sysroot_path(sysroot_arg: Option<&str>) -> Option<PathBuf> {
    let path = sysroot_arg
        .map(PathBuf::from)
        .or_else(|| std::option_env!("SYSROOT").map(PathBuf::from))
        .or_else(|| {
            let home = std::option_env!("RUSTUP_HOME");
            let toolchain = std::option_env!("RUSTUP_TOOLCHAIN");
            toolchain_path(home.map(String::from), toolchain.map(String::from))
        });
    tracing::debug!(?path, ?sysroot_arg, "Sysroot path.");
    path
}

#[cfg(test)]
mod args_test {
    use super::*;
    use crate::parser;
    #[cfg(unix)]
    #[test]
    #[should_panic]
    fn test_invalid_arg_fails() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStrExt;

        // The value 0x80 is an invalid character.
        let source = [0x68, 0x65, 0x6C, 0x6C, 0x80];
        let os_str = OsStr::from_bytes(&source[..]);
        assert_eq!(os_str.to_str(), None);

        let matches = parser::parser().get_matches_from(vec![
            OsString::from("--sysroot").as_os_str(),
            OsString::from("any").as_os_str(),
            os_str,
        ]);
        generate_rustc_args(&matches);
    }
}
