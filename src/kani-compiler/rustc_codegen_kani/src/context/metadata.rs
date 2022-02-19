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
}

/// We emit this struct for every unwind we find

#[derive(Serialize)]
pub struct UnwindMetadata {
    /// The name of the function in the CBMC symbol table, being used as a unique identifer.
    pub mangled_name: String,
    /// The value of the unwind attribute that the user wants to set
    pub unwind_value: u32,
}

/// The structure of `.kani-metadata.json` files, which are emitted for each crate
#[derive(Serialize)]
pub struct KaniMetadata {
    pub proof_harnesses: Vec<HarnessMetadata>,
    pub unwind_metadata: Vec<UnwindMetadata>,
}
