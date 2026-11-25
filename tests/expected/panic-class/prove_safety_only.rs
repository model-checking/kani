// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z unstable-options --prove-safety-only

//! `--prove-safety-only` assumes that no panic occurs, but still checks `kani::assert` and
//! postconditions (#5212).

#[kani::proof]
fn rust_assert() {
    let x: u8 = kani::any();
    assert!(x != 3);
}

#[kani::proof]
fn kani_assert() {
    let x: u8 = kani::any();
    kani::assert(x != 3, "kani check");
}

#[kani::ensures(|result: &u8| *result == x)]
fn wrong(x: u8) -> u8 {
    x.wrapping_add(1)
}

#[kani::proof_for_contract(wrong)]
fn postcondition() {
    wrong(kani::any());
}
