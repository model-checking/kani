// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing --no-assert-contracts

#[kani::requires(x < 8)]
#[kani::ensures(|result| *result == x + 1)]
fn f(x: u8) -> u8 {
    x + 1
}

#[kani::requires(x < 4)]
#[kani::ensures(|result| *result == x + 3)]
fn replacement(x: u8) -> u8 {
    x + 2
}

#[kani::proof]
#[kani::stub(f, replacement)]
fn ordinary_stub_original_mode() {
    assert_eq!(f(8), 10);
}

#[kani::proof_for_contract(f)]
fn provider() {
    f(kani::any());
}

#[kani::proof]
#[kani::stub_verified(f)]
fn verified_control() {
    assert_eq!(f(7), 8);
}

#[kani::proof_for_contract(replacement)]
#[kani::should_panic]
fn replacement_provider() {
    replacement(0);
}
