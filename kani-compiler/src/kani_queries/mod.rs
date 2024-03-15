// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use cbmc::{InternString, InternedString};
use kani_metadata::AssignsContract;
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
    pub harnesses_info: HashMap<InternedString, PathBuf>,
    modifies_contracts: HashMap<InternedString, AssignsContract>,
}

impl QueryDb {
    pub fn new() -> Arc<Mutex<QueryDb>> {
        Arc::new(Mutex::new(QueryDb::default()))
    }

    /// Get the definition hash for all harnesses that are being compiled in this compilation stage.
    pub fn target_harnesses(&self) -> Vec<InternedString> {
        self.harnesses_info.keys().cloned().collect()
    }

    /// Get the model path for a given harness.
    pub fn harness_model_path(&self, harness: &String) -> Option<&PathBuf> {
        self.harnesses_info.get(&harness.intern())
    }

    pub fn set_args(&mut self, args: Arguments) {
        self.args = Some(args);
    }

    pub fn args(&self) -> &Arguments {
        self.args.as_ref().expect("Arguments have not been initialized")
    }

    /// Register that a CBMC-level `assigns` contract for a function that is
    /// called from this harness.
    pub fn register_assigns_contract(
        &mut self,
        harness_name: InternedString,
        contract: AssignsContract,
    ) {
        let replaced = self.modifies_contracts.insert(harness_name, contract);
        assert!(
            replaced.is_none(),
            "Invariant broken, tried adding second modifies contracts to: {harness_name}",
        )
    }

    /// Lookup all CBMC-level `assigns` contract were registered with
    /// [`Self::add_assigns_contract`].
    pub fn assigns_contracts(&self) -> impl Iterator<Item = (&InternedString, &AssignsContract)> {
        self.modifies_contracts.iter()
    }
}
