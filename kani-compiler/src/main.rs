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
mod kani_middle;
mod parser;
mod session;
mod unsound_experiments;

use crate::kani_middle::stubbing;
use crate::parser::KaniCompilerParser;
use crate::session::init_session;
use clap::ArgMatches;
use kani_queries::{QueryDb, ReachabilityType, UserInput};
use rustc_data_structures::fx::FxHashMap;
use rustc_driver::{Callbacks, RunCompiler};
use rustc_hir::definitions::DefPathHash;
use rustc_interface::Config;
use rustc_session::config::ErrorOutputType;
use session::json_panic_hook;
use std::env;
use std::ffi::OsStr;
use std::rc::Rc;

/// Main function. Configure arguments and run the compiler.
fn main() -> Result<(), &'static str> {
    let args = parser::command_arguments(&env::args().collect());
    let matches = parser::parser().get_matches_from(args);
    init_session(&matches);

    // Configure queries.
    let mut queries = QueryDb::default();
    queries.set_emit_vtable_restrictions(matches.get_flag(parser::RESTRICT_FN_PTRS));
    queries.set_check_assertion_reachability(matches.get_flag(parser::ASSERTION_REACH_CHECKS));
    queries.set_output_pretty_json(matches.get_flag(parser::PRETTY_OUTPUT_FILES));
    queries.set_ignore_global_asm(matches.get_flag(parser::IGNORE_GLOBAL_ASM));
    queries.set_reachability_analysis(matches.reachability_type());
    #[cfg(feature = "unsound_experiments")]
    crate::unsound_experiments::arg_parser::add_unsound_experiment_args_to_queries(
        &mut queries,
        &matches,
    );

    // Generate rustc args.
    let mut rustc_args = generate_rustc_args(&matches);

    // If appropriate, collect and set the stub mapping.
    if matches.get_flag(parser::ENABLE_STUBBING)
        && queries.get_reachability_analysis() == ReachabilityType::Harnesses
    {
        queries.set_stubbing_enabled(true);
        let all_stub_mappings =
            stubbing::collect_stub_mappings(&rustc_args).or(Err("Failed to compile crate"))?;
        let harness = matches.get_one::<String>(parser::HARNESS).unwrap();
        let mapping = find_harness_stub_mapping(harness, all_stub_mappings).unwrap_or_default();
        rustc_args.push(stubbing::mk_rustc_arg(mapping));
    }

    // Configure and run compiler.
    let mut callbacks = KaniCallbacks {};
    let mut compiler = RunCompiler::new(&rustc_args, &mut callbacks);
    if matches.get_flag("goto-c") {
        if cfg!(feature = "cprover") {
            compiler.set_make_codegen_backend(Some(Box::new(move |_cfg| {
                Box::new(codegen_cprover_gotoc::GotocCodegenBackend::new(&Rc::new(queries)))
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
impl Callbacks for KaniCallbacks {
    fn config(&mut self, config: &mut Config) {
        if matches!(config.opts.error_format, ErrorOutputType::Json { .. }) {
            json_panic_hook();
        }
    }
}

/// Generate the arguments to pass to rustc_driver.
fn generate_rustc_args(args: &ArgMatches) -> Vec<String> {
    let mut rustc_args = vec![String::from("rustc")];
    if args.get_flag(parser::RUSTC_VERSION) {
        rustc_args.push(String::from("--version"))
    }

    if args.get_flag(parser::JSON_OUTPUT) {
        rustc_args.push(String::from("--error-format=json"));
    }

    if let Some(extra_flags) = args.get_raw(parser::RUSTC_OPTIONS) {
        extra_flags.for_each(|arg| rustc_args.push(convert_arg(arg)));
    }
    tracing::debug!(?rustc_args, "Compile");
    rustc_args
}

/// Convert an argument from OsStr to String.
/// If conversion fails, panic with a custom message.
fn convert_arg(arg: &OsStr) -> String {
    arg.to_str().expect(format!("[Error] Cannot parse argument \"{arg:?}\".").as_str()).to_string()
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
            OsString::from("kani-compiler").as_os_str(),
            OsString::from("--sysroot").as_os_str(),
            OsString::from("any").as_os_str(),
            os_str,
        ]);
        generate_rustc_args(&matches);
    }
}
