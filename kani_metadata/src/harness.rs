// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};

/// We emit this structure for each annotated proof harness (`#[kani::proof]`) we find
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub unwind_value: Option<u32>,
}
