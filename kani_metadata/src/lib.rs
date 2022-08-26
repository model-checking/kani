// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod harness;
mod vtable;

pub use harness::*;
pub use vtable::*;

use serde::{Deserialize, Serialize};

/// The structure of `.kani-metadata.json` files, which are emitted for each crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaniMetadata {
    /// The proof harnesses (#[kani::proof]) found in this crate.
    pub proof_harnesses: Vec<HarnessMetadata>,
    /// The features found in this crate that Kani does not support.
    /// (These general translate to `assert(false)` so we can still attempt verification.)
    pub unsupported_features: Vec<UnsupportedFeature>,
    /// If crates are built in test-mode, then test harnesses will be recorded here.
    pub test_harnesses: Vec<HarnessMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedFeature {
    /// A string identifying the feature.
    pub feature: String,
    /// A list of locations (file, line) where this unsupported feature can be found.
    pub locations: Vec<(String, String)>,
}
