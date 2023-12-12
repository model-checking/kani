// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use rustc_hir::definitions::DefPathHash;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::args::Arguments;

/// This structure should only be used behind a synchronized reference or a snapshot.
#[derive(Debug, Default, Clone)]
pub struct QueryDb {
    args: Option<Arguments>,
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

    pub fn set_args(&mut self, args: Arguments) {
        self.args = Some(args);
    }

    pub fn args(&self) -> &Arguments {
        self.args.as_ref().expect("Arguments have not been initialized")
    }
}
