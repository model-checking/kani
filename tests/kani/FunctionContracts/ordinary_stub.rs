// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing

// Ordinary replacements supply the behavior of contracted targets (issue #4591).

#[kani::requires(x < 8)]
#[kani::ensures(|result| *result == x + 1)]
fn f(x: u8) -> u8 {
    x + 1
}

fn g(x: u8) -> u8 {
    x + 2
}

#[kani::proof]
#[kani::stub(f, g)]
#[kani::stub(requires_only, g)]
#[kani::stub(ensures_only, g)]
#[kani::stub(recursive, g)]
#[kani::stub(generic, generic_replacement)]
#[kani::stub(uncontracted, requires_only)]
fn ordinary_stub() {
    assert_eq!(f(8), 10);
    assert_eq!(requires_only(8), 10);
    assert_eq!(ensures_only(8), 10);
    assert_eq!(recursive(8), 10);
    assert_eq!(generic(1u8, 2u8), 2);
    assert_eq!(generic(3u16, 4u16), 4);
    assert_eq!(generic(5u8, 5u8), 5);
    assert_eq!(uncontracted(8), 10);
    assert_eq!(contracted_replacement(8), 11);
}

#[kani::requires(x < 8)]
fn requires_only(x: u8) -> u8 {
    x + 1
}

#[kani::ensures(|result| *result == x)]
fn ensures_only(x: u8) -> u8 {
    x
}

#[kani::requires(x < 4)]
#[kani::ensures(|result| *result == x)]
#[kani::recursion]
fn recursive(x: u8) -> u8 {
    if x == 0 { 0 } else { recursive(x - 1) + 1 }
}

#[kani::requires(x != y)]
#[kani::ensures(|result| *result == x)]
fn generic<T: Copy + PartialEq>(x: T, y: T) -> T {
    x
}

fn generic_replacement<T: Copy + PartialEq>(_x: T, y: T) -> T {
    y
}

fn uncontracted(x: u8) -> u8 {
    x
}

#[kani::requires(x < 16)]
#[kani::ensures(|result| *result == x + 3)]
fn contracted_replacement(x: u8) -> u8 {
    x + 3
}

#[kani::proof_for_contract(contracted_replacement)]
fn replacement_provider() {
    contracted_replacement(kani::any());
}

#[kani::proof]
#[kani::stub(f, contracted_replacement)]
#[kani::stub_verified(contracted_replacement)]
fn replacement_verified_contract() {
    assert_eq!(f(8), 11);
}

#[kani::requires(x < 16)]
#[kani::ensures(|result| *result == x + 3)]
fn replacement_caller(x: u8) -> u8 {
    f(x)
}

#[kani::proof_for_contract(replacement_caller)]
#[kani::stub(f, contracted_replacement)]
fn replacement_contract_target() {
    replacement_caller(kani::any());
}
