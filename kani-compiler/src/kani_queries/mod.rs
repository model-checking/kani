// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define the communication between KaniCompiler and the codegen implementation.

use cbmc::{InternString, InternedString};
use kani_metadata::AssignsContract;
use std::fmt::{Display, Formatter, Write};
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

    /// Lookup if a CBMC-level `assigns` contract was registered for this
    /// harness with [`Self::add_assigns_contract`].
    ///
    /// This removes the contract from the registry and is intended to be used in
    /// conjunction with [`Self::assert_assigns_contracts_retrieved`] to uphold
    /// the invariant that each contract has been handled and only handled once.
    pub fn assigns_contract_for(
        &mut self,
        harness_name: InternedString,
    ) -> Option<AssignsContract> {
        self.modifies_contracts.remove(&harness_name)
    }

    /// Assert that the contract registry is empty. See [`Self::assigns_contract_for`]
    pub fn assert_assigns_contracts_retrieved(&self) {
        assert!(
            self.modifies_contracts.is_empty(),
            "Invariant broken: The modifies contracts for {} have not been retrieved",
            PrintList(self.modifies_contracts.keys())
        )
    }
}

struct PrintList<I>(I);

impl<E: Display, I: Iterator<Item = E> + Clone> Display for PrintList<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        let mut is_first = true;
        for e in self.0.clone() {
            if is_first {
                f.write_str(", ")?;
                is_first = false;
            }
            e.fmt(f)?;
        }
        f.write_char(']')
    }
}
