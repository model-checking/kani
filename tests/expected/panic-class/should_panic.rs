// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! `should_panic` accepts panics, including `assert!` and arithmetic overflow, but not failed
//! Kani checks such as `kani::assert` or a postcondition (#5212).

#[kani::proof]
#[kani::should_panic]
fn rust_assert() {
    let x: u8 = kani::any();
    assert!(x != 3);
}

#[kani::proof]
#[kani::should_panic]
fn overflow() {
    let x: u8 = kani::any();
    let _ = x + 1;
}

#[kani::proof]
#[kani::should_panic]
fn kani_assert() {
    let x: u8 = kani::any();
    kani::assert(x != 3, "kani check");
}

#[kani::ensures(|result: &u8| *result == x)]
fn wrong(x: u8) -> u8 {
    x.wrapping_add(1)
}

#[kani::proof_for_contract(wrong)]
#[kani::should_panic]
fn postcondition() {
    wrong(kani::any());
}
