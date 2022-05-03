// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod harness;
mod vtable;

pub use harness::*;
pub use vtable::*;

use serde::{Deserialize, Serialize};

/// The structure of `.kani-metadata.json` files, which are emitted for each crate
#[derive(Serialize, Deserialize)]
pub struct KaniMetadata {
    pub proof_harnesses: Vec<HarnessMetadata>,
}
