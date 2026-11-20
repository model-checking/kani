// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::requires(x < 100)]
#[kani::ensures(|r: &u8| *r == x + 1)]
fn inc(x: u8) -> u8 {
    x + 1
}

#[kani::proof_for_contract(inc)]
fn check_inc() {
    inc(kani::any());
}
