// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use rustc_hir::definitions::DefPathHash;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

#[derive(Debug, Default, Clone, Copy, AsRefStr, EnumString, EnumVariantNames, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum BackendOption {
    /// Boogie backend
    Boogie,

    /// CProver (Goto) backend
    CProver,

    /// Backend option was not explicitly set
    #[default]
    None,
}

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
#[derive(Debug, Default, Clone)]
pub struct QueryDb {
    pub backend: BackendOption,
    pub check_assertion_reachability: bool,
    pub emit_vtable_restrictions: bool,
    pub output_pretty_json: bool,
    pub ignore_global_asm: bool,
    /// When set, instructs the compiler to produce the symbol table for CBMC in JSON format and use symtab2gb.
    pub write_json_symtab: bool,
    pub reachability_analysis: ReachabilityType,
    pub stubbing_enabled: bool,
    pub unstable_features: Vec<String>,

    /// Information about all target harnesses.
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
}
