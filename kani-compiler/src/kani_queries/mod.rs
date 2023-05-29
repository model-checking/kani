// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use std::sync::{Arc, Mutex};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

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
    unstable_features: Vec<String>,
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
            unstable_features: vec![],
        }))
    }
}

impl QueryDb {
    pub fn set_emit_vtable_restrictions(&mut self, restrictions: bool) {
        self.emit_vtable_restrictions = restrictions;
    }

    pub fn get_emit_vtable_restrictions(&self) -> bool {
        self.emit_vtable_restrictions
    }

    pub fn set_check_assertion_reachability(&mut self, reachability: bool) {
        self.check_assertion_reachability = reachability;
    }

    pub fn get_check_assertion_reachability(&self) -> bool {
        self.check_assertion_reachability
    }

    pub fn set_output_pretty_json(&mut self, pretty_json: bool) {
        self.json_pretty_print = pretty_json;
    }

    pub fn get_output_pretty_json(&self) -> bool {
        self.json_pretty_print
    }

    pub fn set_ignore_global_asm(&mut self, global_asm: bool) {
        self.ignore_global_asm = global_asm;
    }

    pub fn get_ignore_global_asm(&self) -> bool {
        self.ignore_global_asm
    }

    pub fn set_reachability_analysis(&mut self, reachability: ReachabilityType) {
        self.reachability_analysis = reachability;
    }

    pub fn get_reachability_analysis(&self) -> ReachabilityType {
        self.reachability_analysis
    }

    pub fn set_stubbing_enabled(&mut self, stubbing_enabled: bool) {
        self.stubbing_enabled = stubbing_enabled;
    }

    pub fn get_stubbing_enabled(&self) -> bool {
        self.stubbing_enabled
    }

    pub fn set_write_json_symtab(&mut self, write_json_symtab: bool) {
        self.write_json_symtab = write_json_symtab;
    }

    pub fn get_write_json_symtab(&self) -> bool {
        self.write_json_symtab
    }

    pub fn set_unstable_features(&mut self, features: &[String]) {
        self.unstable_features = Vec::from_iter(features.iter().cloned());
    }

    pub fn get_unstable_features(&self) -> &[String] {
        &self.unstable_features
    }
}
