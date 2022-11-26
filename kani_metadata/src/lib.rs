// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};

pub use artifact::ArtifactType;
pub use harness::*;
pub use vtable::*;

pub mod artifact;
mod harness;
mod vtable;

/// The structure of `.kani-metadata.json` files, which are emitted for each crate
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedFeature {
    // We could replace this with an enum: https://github.com/model-checking/kani/issues/1765
    /// A string identifying the feature.
    pub feature: String,
    /// A list of locations (file, line) where this unsupported feature can be found.
    pub locations: Vec<(String, String)>,
}
