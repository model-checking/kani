// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(feature = "unsound_experiments"))]
use std::sync::Mutex;
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

#[cfg(feature = "unsound_experiments")]
mod unsound_experiments;

#[cfg(feature = "unsound_experiments")]
use {
    crate::unsound_experiments::UnsoundExperiments,
    std::sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Copy, AsRefStr, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum ReachabilityType {
    /// Start the cross-crate reachability analysis from all harnesses in the local crate.
    Harnesses,
    /// Use standard rustc monomorphizer algorithm.
    Legacy,
    /// Don't perform any reachability analysis. This will skip codegen for this crate.
    None,
    /// Start the cross-crate reachability analysis from all public functions in the local crate.
    PubFns,
    /// Start the cross-crate reachability analysis from all *test* (i.e. `#[test]`) harnesses in the local crate.
    Tests,
}

impl Default for ReachabilityType {
    fn default() -> Self {
        ReachabilityType::None
    }
}

pub trait UserInput {
    fn set_symbol_table_passes(&mut self, passes: Vec<String>);
    fn get_symbol_table_passes(&self) -> Vec<String>;

    fn set_emit_vtable_restrictions(&mut self, restrictions: bool);
    fn get_emit_vtable_restrictions(&self) -> bool;

    fn set_check_assertion_reachability(&mut self, reachability: bool);
    fn get_check_assertion_reachability(&self) -> bool;

    fn set_output_pretty_json(&mut self, pretty_json: bool);
    fn get_output_pretty_json(&self) -> bool;

    fn set_ignore_global_asm(&mut self, global_asm: bool);
    fn get_ignore_global_asm(&self) -> bool;

    fn set_reachability_analysis(&mut self, reachability: ReachabilityType);
    fn get_reachability_analysis(&self) -> ReachabilityType;

    fn set_stubbing_enabled(&mut self, stubbing_enabled: bool);
    fn get_stubbing_enabled(&self) -> bool;

    #[cfg(feature = "unsound_experiments")]
    fn get_unsound_experiments(&self) -> Arc<Mutex<UnsoundExperiments>>;
}

#[derive(Debug, Default)]
pub struct QueryDb {
    check_assertion_reachability: AtomicBool,
    emit_vtable_restrictions: AtomicBool,
    symbol_table_passes: Vec<String>,
    json_pretty_print: AtomicBool,
    ignore_global_asm: AtomicBool,
    reachability_analysis: Mutex<ReachabilityType>,
    stubbing_enabled: bool,
    #[cfg(feature = "unsound_experiments")]
    unsound_experiments: Arc<Mutex<UnsoundExperiments>>,
}

impl UserInput for QueryDb {
    fn set_symbol_table_passes(&mut self, passes: Vec<String>) {
        self.symbol_table_passes = passes;
    }

    fn get_symbol_table_passes(&self) -> Vec<String> {
        self.symbol_table_passes.clone()
    }

    fn set_emit_vtable_restrictions(&mut self, restrictions: bool) {
        self.emit_vtable_restrictions.store(restrictions, Ordering::Relaxed);
    }

    fn get_emit_vtable_restrictions(&self) -> bool {
        self.emit_vtable_restrictions.load(Ordering::Relaxed)
    }

    fn set_check_assertion_reachability(&mut self, reachability: bool) {
        self.check_assertion_reachability.store(reachability, Ordering::Relaxed);
    }

    fn get_check_assertion_reachability(&self) -> bool {
        self.check_assertion_reachability.load(Ordering::Relaxed)
    }

    fn set_output_pretty_json(&mut self, pretty_json: bool) {
        self.json_pretty_print.store(pretty_json, Ordering::Relaxed);
    }

    fn get_output_pretty_json(&self) -> bool {
        self.json_pretty_print.load(Ordering::Relaxed)
    }

    fn set_ignore_global_asm(&mut self, global_asm: bool) {
        self.ignore_global_asm.store(global_asm, Ordering::Relaxed);
    }

    fn get_ignore_global_asm(&self) -> bool {
        self.ignore_global_asm.load(Ordering::Relaxed)
    }

    fn set_reachability_analysis(&mut self, reachability: ReachabilityType) {
        *self.reachability_analysis.get_mut().unwrap() = reachability;
    }

    fn get_reachability_analysis(&self) -> ReachabilityType {
        *self.reachability_analysis.lock().unwrap()
    }

    fn set_stubbing_enabled(&mut self, stubbing_enabled: bool) {
        self.stubbing_enabled = stubbing_enabled;
    }

    fn get_stubbing_enabled(&self) -> bool {
        self.stubbing_enabled
    }

    #[cfg(feature = "unsound_experiments")]
    fn get_unsound_experiments(&self) -> Arc<Mutex<UnsoundExperiments>> {
        self.unsound_experiments.clone()
    }
}
