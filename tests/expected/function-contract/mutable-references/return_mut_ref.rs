// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

#[kani::requires(*b > 0 && *b < a)]
#[kani::ensures(|result| **result <= a)]
#[kani::modifies(b)]
fn divide_by(a: u8, b: &mut u8) -> &mut u8 {
    *b = a / *b;
    b
}

#[kani::proof_for_contract(divide_by)]
fn divide_by_harness() {
    divide_by(kani::any(), &mut kani::any());
}
