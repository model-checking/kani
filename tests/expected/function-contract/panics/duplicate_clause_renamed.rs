// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! A second clause of the same panic kind is rejected even through a renamed import.

extern crate kani;
use kani::panics_only_if as p;
#[p(x == 0)]
#[p(x == 1)]
fn d(x: u8) -> u8 {
    100 / x
}
#[kani::proof_for_contract(d)]
fn check() {
    d(kani::any());
}
