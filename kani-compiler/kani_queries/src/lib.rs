// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::{Arc, Mutex};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

#[cfg(feature = "unsound_experiments")]
mod unsound_experiments;

#[cfg(feature = "unsound_experiments")]
use crate::unsound_experiments::UnsoundExperiments;

#[derive(Debug, Default, Clone, Copy, AsRefStr, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum ReachabilityType {
    /// Start the cross-crate reachability analysis from all harnesses in the local crate.
    Harnesses,
    /// Use standard rustc monomorphizer algorithm.
    Legacy,
    /// Don't perform any reachability analysis. This will skip codegen for this crate.
    #[default]
    None,
    /// Start the cross-crate reachability analysis from all public functions in the local crate.
    PubFns,
    /// Start the cross-crate reachability analysis from all *test* (i.e. `#[test]`) harnesses in the local crate.
    Tests,
}

pub trait UserInput {
    fn set_emit_vtable_restrictions(&mut self, restrictions: bool);
    fn get_emit_vtable_restrictions(&self) -> bool;

    fn set_check_assertion_reachability(&mut self, reachability: bool);
    fn get_check_assertion_reachability(&self) -> bool;

    fn set_output_pretty_json(&mut self, pretty_json: bool);
    fn get_output_pretty_json(&self) -> bool;

    fn set_ignore_global_asm(&mut self, global_asm: bool);
    fn get_ignore_global_asm(&self) -> bool;

    fn set_write_json_symtab(&mut self, write_json_symtab: bool);
    fn get_write_json_symtab(&self) -> bool;

    fn set_reachability_analysis(&mut self, reachability: ReachabilityType);
    fn get_reachability_analysis(&self) -> ReachabilityType;

    fn set_stubbing_enabled(&mut self, stubbing_enabled: bool);
    fn get_stubbing_enabled(&self) -> bool;

    #[cfg(feature = "unsound_experiments")]
    fn get_unsound_experiments(&self) -> UnsoundExperiments;
    #[cfg(feature = "unsound_experiments")]
    fn set_unsound_experiments(&mut self, experiments: UnsoundExperiments);
}

/// This structure should only be used behind a synchronized reference or a snapshot.
#[derive(Debug, Clone)]
pub struct QueryDb {
    check_assertion_reachability: bool,
    emit_vtable_restrictions: bool,
    json_pretty_print: bool,
    ignore_global_asm: bool,
    /// When set, instructs the compiler to produce the symbol table for CBMC in JSON format and use symtab2gb.
    write_json_symtab: bool,
    reachability_analysis: ReachabilityType,
    stubbing_enabled: bool,
    #[cfg(feature = "unsound_experiments")]
    unsound_experiments: UnsoundExperiments,
}

impl QueryDb {
    pub fn new() -> Arc<Mutex<QueryDb>> {
        Arc::new(Mutex::new(QueryDb {
            check_assertion_reachability: false,
            emit_vtable_restrictions: false,
            json_pretty_print: false,
            ignore_global_asm: false,
            write_json_symtab: false,
            reachability_analysis: ReachabilityType::None,
            stubbing_enabled: false,
            #[cfg(feature = "unsound_experiments")]
            unsound_experiments: unsound_experiments::UnsoundExperiments { zero_init_vars: false },
        }))
    }
}

impl UserInput for QueryDb {
    fn set_emit_vtable_restrictions(&mut self, restrictions: bool) {
        self.emit_vtable_restrictions = restrictions;
    }

    fn get_emit_vtable_restrictions(&self) -> bool {
        self.emit_vtable_restrictions
    }

    fn set_check_assertion_reachability(&mut self, reachability: bool) {
        self.check_assertion_reachability = reachability;
    }

    fn get_check_assertion_reachability(&self) -> bool {
        self.check_assertion_reachability
    }

    fn set_output_pretty_json(&mut self, pretty_json: bool) {
        self.json_pretty_print = pretty_json;
    }

    fn get_output_pretty_json(&self) -> bool {
        self.json_pretty_print
    }

    fn set_ignore_global_asm(&mut self, global_asm: bool) {
        self.ignore_global_asm = global_asm;
    }

    fn get_ignore_global_asm(&self) -> bool {
        self.ignore_global_asm
    }

    fn set_reachability_analysis(&mut self, reachability: ReachabilityType) {
        self.reachability_analysis = reachability;
    }

    fn get_reachability_analysis(&self) -> ReachabilityType {
        self.reachability_analysis
    }

    fn set_stubbing_enabled(&mut self, stubbing_enabled: bool) {
        self.stubbing_enabled = stubbing_enabled;
    }

    fn get_stubbing_enabled(&self) -> bool {
        self.stubbing_enabled
    }

    fn set_write_json_symtab(&mut self, write_json_symtab: bool) {
        self.write_json_symtab = write_json_symtab;
    }

    fn get_write_json_symtab(&self) -> bool {
        self.write_json_symtab
    }

    #[cfg(feature = "unsound_experiments")]
    fn get_unsound_experiments(&self) -> UnsoundExperiments {
        self.unsound_experiments
    }

    #[cfg(feature = "unsound_experiments")]
    fn set_unsound_experiments(&mut self, experiments: UnsoundExperiments) {
        self.unsound_experiments = experiments
    }
}
