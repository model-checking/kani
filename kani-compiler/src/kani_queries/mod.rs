// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use cbmc::InternedString;
use kani_metadata::AssignsContract;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::args::Arguments;

/// This structure should only be used behind a synchronized reference or a snapshot.
#[derive(Debug, Default, Clone)]
pub struct QueryDb {
    args: Option<Arguments>,
    modifies_contracts: HashMap<InternedString, AssignsContract>,
}

impl QueryDb {
    pub fn new() -> Arc<Mutex<QueryDb>> {
        Arc::new(Mutex::new(QueryDb::default()))
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
}
