// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing --no-assert-contracts

#[kani::requires(x < 8)]
#[kani::ensures(|result| *result == x + 1)]
fn f(x: u8) -> u8 {
    x + 1
}

#[kani::requires(x < 4)]
#[kani::ensures(|result| *result == x + 2)]
fn replacement(x: u8) -> u8 {
    x + 2
}

#[kani::proof]
#[kani::stub(f, replacement)]
fn ordinary_stub_original_mode() {
    assert_eq!(f(8), 10);
}
