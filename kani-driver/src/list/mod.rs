// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Implements the list subcommand logic

use kani_metadata::ContractedFunction;
use std::collections::{BTreeMap, BTreeSet};

pub mod collect_metadata;
pub mod output;

type FileName = String;
type HarnessName = String;

/// Metadata for the list subcommand for a given crate.
/// It is important that crate_name is the first field so that `Ord` orders two ListMetadata objects by crate name.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct ListMetadata {
    crate_name: String,
    // Files mapped to their #[kani::proof] harnesses
    standard_harnesses: BTreeMap<FileName, BTreeSet<HarnessName>>,
    // Total number of #[kani::proof] harnesses
    standard_harnesses_count: usize,
    // Files mapped to their #[kani::proof_for_contract] harnesses
    contract_harnesses: BTreeMap<FileName, BTreeSet<HarnessName>>,
    // Total number of #[kani:proof_for_contract] harnesses
    contract_harnesses_count: usize,
    // Set of all functions under contract
    contracted_functions: BTreeSet<ContractedFunction>,
}

/// Given a collection of ListMetadata objects, merge them into a single ListMetadata object.
pub fn merge_list_metadata<T>(collection: T) -> ListMetadata
where
    T: Extend<ListMetadata>,
    T: IntoIterator<Item = ListMetadata>,
{
    collection
        .into_iter()
        .reduce(|mut acc, item| {
            acc.standard_harnesses.extend(item.standard_harnesses);
            acc.standard_harnesses_count += item.standard_harnesses_count;
            acc.contract_harnesses.extend(item.contract_harnesses);
            acc.contract_harnesses_count += item.contract_harnesses_count;
            acc.contracted_functions.extend(item.contracted_functions);
            acc
        })
        .expect("Cannot merge empty collection of ListMetadata objects")
}
