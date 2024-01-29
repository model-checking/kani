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

    pub fn add_modifies_contract(&mut self, name: InternedString, contract: AssignsContract) {
        assert!(
            self.modifies_contracts.insert(name, contract).is_none(),
            "Invariant broken, tried adding second modifies contracts to: {name}",
        )
    }

    pub fn get_modifies_contracts(&mut self, key: &str) -> Option<AssignsContract> {
        self.modifies_contracts.remove(&key.intern())
    }

    pub fn assert_modifies_contracts_received(&self) {
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
