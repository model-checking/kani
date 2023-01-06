// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This harness does not exist unless the feature is enabled
//! The expected test ensures that gets passed down to cargo through Kani
//!
//! We're testing enabling this feature by embedding `--features proof` via the Kani
//! options in the Cargo.toml

#[cfg(feature = "proof")]
#[kani::proof]
fn trivial_success() {}
