// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing

#[kani::requires(x < 8)]
#[kani::ensures(|result| *result == x + 1)]
fn f(x: u8) -> u8 {
    x + 1
}

fn g(x: u8) -> u8 {
    x + 2
}

#[kani::proof_for_contract(f)]
fn provider() {
    f(kani::any());
}

#[kani::proof]
#[kani::stub(f, g)]
fn replacement_overflow() {
    f(u8::MAX);
}

#[kani::proof]
#[kani::stub_verified(f)]
fn verified_precondition() {
    f(8);
}

#[kani::proof]
#[kani::stub_verified(f)]
fn verified_postcondition() {
    assert_eq!(f(7), 9);
}

#[kani::ensures(|result| *result == x + 2)]
fn bad_contract(x: u8) -> u8 {
    x
}

#[kani::proof_for_contract(bad_contract)]
fn provider_postcondition() {
    bad_contract(0);
}

#[kani::proof]
#[kani::stub(f, g)]
fn unstubbed_postcondition() {
    bad_contract(0);
}

#[kani::proof]
#[kani::stub(bad_contract, g)]
fn unstubbed_precondition() {
    f(8);
}

#[kani::proof]
#[kani::stub(f, bad_contract)]
fn replacement_postcondition() {
    f(0);
}

// The replacement's postcondition fails for input 1.
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
    replacement_caller(1);
}

#[kani::requires(x < 4)]
fn restricted_replacement(x: u8) -> u8 {
    x
}

#[kani::proof]
#[kani::stub(f, restricted_replacement)]
fn replacement_precondition() {
    f(4);
}

#[kani::requires(x < 8)]
fn requires_only(x: u8) -> u8 {
    x + 1
}

#[kani::proof_for_contract(requires_only)]
fn requires_only_provider() {
    requires_only(kani::any());
}

#[kani::proof]
#[kani::stub_verified(requires_only)]
fn requires_only_verified_precondition() {
    requires_only(8);
}

#[kani::proof]
#[kani::stub_verified(requires_only)]
fn requires_only_verified_result() {
    assert_eq!(requires_only(7), 8);
}
