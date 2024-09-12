// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate clap;

use cli_table::Table;
use std::{collections::HashSet, fmt::Display, path::PathBuf};
use serde::{Deserialize, Serialize};

pub use artifact::ArtifactType;
pub use cbmc_solver::CbmcSolver;
pub use harness::*;
pub use vtable::*;

pub mod artifact;
mod cbmc_solver;
mod harness;
pub mod unstable;
mod vtable;

pub use unstable::{EnabledUnstableFeatures, UnstableFeature};

/// The structure of `.kani-metadata.json` files, which are emitted for each crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaniMetadata {
    /// The crate name from which this metadata was extracted.
    pub crate_name: String,
    /// The proof harnesses (`#[kani::proof]`) found in this crate.
    pub proof_harnesses: Vec<HarnessMetadata>,
    /// The features found in this crate that Kani does not support.
    /// (These general translate to `assert(false)` so we can still attempt verification.)
    pub unsupported_features: Vec<UnsupportedFeature>,
    /// If crates are built in test-mode, then test harnesses will be recorded here.
    pub test_harnesses: Vec<HarnessMetadata>,
    /// The functions with contracts in this crate
    pub contracted_functions: Vec<ContractedFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Table)]
pub struct ContractedFunction {
    /// The fully qualified name the user gave to the function (i.e. includes the module path).
    pub pretty_name: String,
    /// The (currently full-) path to the file this function was declared within.
    #[table(skip)]
    pub original_file: String,
    /// The number of contracts applied to this function
    pub contracts_count: usize,
    /// The pretty names of the proof harnesses (`#[kani::proof_for_contract]`) for this function
    #[table(display_fn = "print_contract_harnesses")]
    pub harnesses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedFeature {
    // We could replace this with an enum: https://github.com/model-checking/kani/issues/1765
    /// A string identifying the feature.
    pub feature: String,
    /// A list of locations (file, line) where this unsupported feature can be found.
    pub locations: HashSet<Location>,
}

/// The location in a file
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Location {
    pub filename: String,
    pub start_line: u64,
}

/// We stub artifacts with the path to a KaniMetadata file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerArtifactStub {
    pub metadata_path: PathBuf,
}

fn print_contract_harnesses(harnesses: &Vec<String>) -> impl Display {
    let joined = harnesses.join("\n");
    if joined.is_empty() {
        "NONE".to_string()
    } else {
        joined
    }
}
