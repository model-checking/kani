// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! A function may have at most one clause of each panic kind.

#[kani::panics_only_if(x == 0)]
#[kani::panics_only_if(x == 1)]
fn divide(x: u8) -> u8 {
    100 / x
}

#[kani::panics_if(x == 0)]
#[kani::panics_if(x == 1)]
fn divide_exact(x: u8) -> u8 {
    100 / x
}
