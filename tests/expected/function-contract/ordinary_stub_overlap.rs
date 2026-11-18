// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing

// Explicit contract modes select the original target during ordinary stubbing.

#[kani::requires(x < 8)]
#[kani::ensures(|result| *result == x + 1)]
fn f(x: u8) -> u8 {
    x + 1
}

fn g(x: u8) -> u8 {
    x + 2
}

#[kani::proof_for_contract(f)]
#[kani::stub_verified(f)]
#[kani::stub(f, g)]
fn overlapping_contract_target() {
    f(kani::any());
}
