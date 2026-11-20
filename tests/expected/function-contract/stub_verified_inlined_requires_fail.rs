// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Zfunction-contracts -Zstubbing
// compile-flags: -Copt-level=2
//
//! Check that the contract precondition is checked for calls the MIR inliner would inline.

#[kani::requires(x < 10)]
#[kani::ensures(|r: &u32| *r == x + 1)]
fn inc(x: u32) -> u32 {
    x + 1
}

fn caller() -> u32 {
    inc(50)
}

#[kani::proof]
#[kani::stub_verified(inc)]
fn uses_contract() {
    let _ = caller();
}

#[kani::proof_for_contract(inc)]
fn check_inc() {
    let _ = inc(kani::any());
}
