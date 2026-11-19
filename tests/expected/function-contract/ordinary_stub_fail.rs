// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing

#[kani::requires(x < 8)]
#[kani::ensures(|result| *result == x + 1)]
fn f(x: u8) -> u8 {
    x + 1
}

// The replacement requires x < 4 and returns x with a zero-result postcondition.
#[kani::requires(x < 4)]
#[kani::ensures(|result| *result == 0)]
fn discriminating_replacement(x: u8) -> u8 {
    x
}

#[kani::ensures(|result| *result == x)]
fn replacement_caller(x: u8) -> u8 {
    f(x)
}

#[kani::proof_for_contract(replacement_caller)]
#[kani::stub(f, discriminating_replacement)]
fn replacement_provider_postcondition() {
    let x: u8 = kani::any();
    kani::assume(x <= 4);
    replacement_caller(x);
}
