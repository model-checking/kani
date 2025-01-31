// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Implements the list subcommand logic

use kani_metadata::ContractedFunction;
use std::collections::{BTreeMap, BTreeSet};

pub mod collect_metadata;
mod output;

struct ListMetadata {
    // Files mapped to their #[kani::proof] harnesses
    standard_harnesses: BTreeMap<String, BTreeSet<String>>,
    // Total number of #[kani::proof] harnesses
    standard_harnesses_count: usize,
    // Files mapped to their #[kani::proof_for_contract] harnesses
    contract_harnesses: BTreeMap<String, BTreeSet<String>>,
    // Total number of #[kani:proof_for_contract] harnesses
    contract_harnesses_count: usize,
    // Set of all functions under contract
    contracted_functions: BTreeSet<ContractedFunction>,
}
