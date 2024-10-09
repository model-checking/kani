// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Implements the list subcommand logic

pub mod collect_metadata;
mod output;

/// Stores the total count of standard harnesses, contract harnesses,
/// and functions under contract across all `KaniMetadata` objects.
struct Totals {
    standard_harnesses: usize,
    contract_harnesses: usize,
    contracted_functions: usize,
}
