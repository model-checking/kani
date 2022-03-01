// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module should be factored out into its own separate crate eventually,
//! but leaving it here for now...

use serde::Serialize;

/// We emit this structure for each annotated proof harness we find
#[derive(Serialize)]
pub struct HarnessMetadata {
    /// The name the user gave to the function
    pub pretty_name: String,
    /// The name of the function in the CBMC symbol table
    pub mangled_name: String,
    /// The (currently full-) path to the file this proof harness was declared within
    pub original_file: String,
    /// The line in that file where the proof harness begins
    pub original_line: String,
    /// Optional data to store unwind value
    pub unwind_value: Option<u128>,
}

/// The structure of `.kani-metadata.json` files, which are emitted for each crate
#[derive(Serialize)]
pub struct KaniMetadata {
    pub proof_harnesses: Vec<HarnessMetadata>,
}
