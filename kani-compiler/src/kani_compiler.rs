// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines all compiler extensions that form the Kani compiler.

use crate::kani_middle::stubbing;
use crate::parser::{self, KaniCompilerParser};
use crate::session::init_session;
use clap::ArgMatches;
use kani_queries::{QueryDb, ReachabilityType, UserInput};
use rustc_data_structures::fx::FxHashMap;
use rustc_driver::{Callbacks, RunCompiler};
use rustc_hir::definitions::DefPathHash;
use rustc_interface::Config;
use rustc_session::config::ErrorOutputType;
use std::env;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[cfg(todo)]
fn config() {}

/// Empty struct since we don't support any callbacks yet.
pub struct KaniCallbacks {
    pub queries: Arc<Mutex<QueryDb>>,
}

/// Use default function implementations.
impl Callbacks for KaniCallbacks {
    fn config(&mut self, config: &mut Config) {
        println!("config: {:?}", config.opts.cg.llvm_args);
        let matches = parser::parser().get_matches_from(&config.opts.cg.llvm_args);
        init_session(&matches, matches!(config.opts.error_format, ErrorOutputType::Json { .. }));

        // Configure queries.
        let queries = &mut (*self.queries.lock().unwrap());
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

        // If appropriate, collect and set the stub mapping.
        if matches.get_flag(parser::ENABLE_STUBBING)
            && queries.get_reachability_analysis() == ReachabilityType::Harnesses
        {
            queries.set_stubbing_enabled(true);
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

fn init_stub(rustc_args: &Vec<String>) -> Result<Option<String>, &str> {
    let all_stub_mappings =
        stubbing::collect_stub_mappings(&rustc_args).or(Err("Failed to compile crate"))?;
    let harness = matches.get_one::<String>(parser::HARNESS).unwrap();
    let mapping = find_harness_stub_mapping(harness, all_stub_mappings).unwrap_or_default();
    rustc_args.push(stubbing::mk_rustc_arg(mapping));
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
