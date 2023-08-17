// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use rustc_hir::definitions::DefPathHash;
use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};
use tracing_subscriber::filter::Directive;

#[derive(Debug, Default, Clone, Copy, AsRefStr, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum ReachabilityType {
    /// Start the cross-crate reachability analysis from all harnesses in the local crate.
    Harnesses,
    /// Don't perform any reachability analysis. This will skip codegen for this crate.
    #[default]
    None,
    /// Start the cross-crate reachability analysis from all public functions in the local crate.
    PubFns,
    /// Start the cross-crate reachability analysis from all *test* (i.e. `#[test]`) harnesses in the local crate.
    Tests,
}

/// This structure should only be used behind a synchronized reference or a snapshot.
#[derive(Debug, Default, Clone, clap::Parser)]
pub struct QueryDb {
    /// Option name used to enable assertion reachability checks.
    #[clap(long = "assertion-reach-checks")]
    pub check_assertion_reachability: bool,
    /// Option name used to enable coverage checks.
    #[clap(long = "coverage-checks")]
    pub check_coverage: bool,
    /// Option name used to dump function pointer restrictions.
    #[clap(long = "restrict-vtable-fn-ptrs")]
    pub emit_vtable_restrictions: bool,
    /// Option name used to use json pretty-print for output files.
    #[clap(long = "pretty-json-files")]
    pub output_pretty_json: bool,
    /// Option used for suppressing global ASM error.
    #[clap(long)]
    pub ignore_global_asm: bool,
    #[clap(long)]
    /// Option used to write JSON symbol tables instead of GOTO binaries.
    ///
    /// When set, instructs the compiler to produce the symbol table for CBMC in JSON format and use symtab2gb.
    pub write_json_symtab: bool,
    /// Option name used to select which reachability analysis to perform.
    #[clap(long = "reachability", default_value = "none")]
    pub reachability_analysis: ReachabilityType,
    #[clap(long = "enable-stubbing")]
    pub stubbing_enabled: bool,
    /// Option name used to define unstable features.
    #[clap(short = 'Z', long = "unstable")]
    pub unstable_features: Vec<String>,
    #[clap(long)]
    /// Option used for building standard library.
    ///
    /// Flag that indicates that we are currently building the standard library.
    /// Note that `kani` library will not be available if this is `true`.
    pub build_std: bool,
    #[clap(long)]
    /// Option name used to set log level.
    pub log_level: Option<Directive>,
    #[clap(long)]
    /// Option name used to set the log output to a json file.
    pub json_output: bool,
    #[clap(long, conflicts_with = "json_output")]
    /// Option name used to force logger to use color output. This doesn't work with --json-output.
    pub color_output: bool,
    #[clap(long)]
    /// Pass the kani version to the compiler to ensure cache coherence.
    check_version: Option<String>,
    #[clap(long)]
    /// A legacy flag that is now ignored.
    goto_c: bool,

    /// Information about all target harnesses.
    #[clap(skip = HashMap::new())]
    pub harnesses_info: HashMap<DefPathHash, PathBuf>,
}

impl QueryDb {
    pub fn new() -> Arc<Mutex<QueryDb>> {
        Arc::new(Mutex::new(QueryDb::default()))
    }

    /// Get the definition hash for all harnesses that are being compiled in this compilation stage.
    pub fn target_harnesses(&self) -> Vec<DefPathHash> {
        self.harnesses_info.keys().cloned().collect()
    }

    /// Get the model path for a given harness.
    pub fn harness_model_path(&self, harness: &DefPathHash) -> Option<&PathBuf> {
        self.harnesses_info.get(harness)
    }

    /// Merges the other (freshly parsed) configuration onto this one. In almost
    /// all cases we just use the values from `other`, except for `harness_info`
    /// which is taken from both.
    ///
    /// Panics if a single def path occurs in both `harness_info`s
    pub fn merge(&mut self, other: &Self) {
        // We use this unpacking here so that if we add new fields to the struct
        // rustc will complain that we're not handling all fields *and* if we
        // forget to set one of the fields the linter will error about an unused
        // value.
        //
        // To ensures these protections stay in place this pattern should always
        // match explicitly on all fields and never use `..`.
        let Self {
            check_assertion_reachability,
            check_coverage,
            emit_vtable_restrictions,
            output_pretty_json,
            ignore_global_asm,
            write_json_symtab,
            reachability_analysis,
            stubbing_enabled,
            unstable_features,
            build_std,
            harnesses_info,
            color_output,
            log_level,
            json_output,
            check_version: _,
            goto_c: _,
        } = self;

        *check_assertion_reachability = other.check_assertion_reachability;
        *check_coverage = other.check_coverage;
        *emit_vtable_restrictions = other.emit_vtable_restrictions;
        *output_pretty_json = other.output_pretty_json;
        *ignore_global_asm = other.ignore_global_asm;
        *write_json_symtab = cfg!(feature = "write_json_symtab") || other.write_json_symtab;
        *reachability_analysis = other.reachability_analysis;
        *stubbing_enabled =
            *reachability_analysis == ReachabilityType::Harnesses && other.stubbing_enabled;
        *unstable_features = other.unstable_features.clone();
        *build_std = other.build_std;
        *color_output = other.color_output;
        *log_level = other.log_level.clone();
        *json_output = other.json_output;

        for (key, value) in &other.harnesses_info {
            match harnesses_info.entry(*key) {
                Entry::Occupied(_) => unreachable!(),
                Entry::Vacant(vac) => vac.insert(value.clone()),
            };
        }
    }
}
