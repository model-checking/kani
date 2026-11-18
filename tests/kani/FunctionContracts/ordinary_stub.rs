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

#[kani::proof_for_contract(f)]
fn provider() {
    f(kani::any());
}

#[kani::proof]
#[kani::stub_verified(f)]
fn verified_control() {
    let x: u8 = kani::any();
    kani::assume(x < 8);
    assert!(f(x) == x + 1);
}

#[kani::proof]
#[kani::stub(f, g)]
fn ordinary_stub() {
    let x: u8 = kani::any();
    kani::assume(x < 8);
    assert!(f(x) == x + 2);
}

#[kani::proof]
#[kani::stub(f, g)]
fn ordinary_stub_outside_target_precondition() {
    assert_eq!(f(8), 10);
}

#[kani::requires(x < 8)]
fn requires_only(x: u8) -> u8 {
    x + 1
}

#[kani::ensures(|result| *result == x)]
fn ensures_only(x: u8) -> u8 {
    x
}

#[kani::proof_for_contract(requires_only)]
fn requires_only_provider() {
    requires_only(kani::any());
}

#[kani::proof]
#[kani::stub_verified(requires_only)]
fn requires_only_verified_control() {
    requires_only(7);
}

#[kani::proof_for_contract(ensures_only)]
fn ensures_only_provider() {
    ensures_only(kani::any());
}

#[kani::proof]
#[kani::stub_verified(ensures_only)]
fn ensures_only_verified_control() {
    assert_eq!(ensures_only(8), 8);
}

#[kani::proof]
#[kani::stub(requires_only, g)]
#[kani::stub(ensures_only, g)]
fn multiple_targets() {
    assert_eq!(requires_only(8), 10);
    assert_eq!(ensures_only(8), 10);
    assert_eq!(f(7), 8);
}

#[kani::requires(x < 4)]
#[kani::ensures(|result| *result == x)]
#[kani::recursion]
fn recursive(x: u8) -> u8 {
    if x == 0 { 0 } else { recursive(x - 1) + 1 }
}

#[kani::proof_for_contract(recursive)]
fn recursive_provider() {
    recursive(kani::any());
}

#[kani::proof]
#[kani::stub_verified(recursive)]
fn recursive_verified_control() {
    assert_eq!(recursive(3), 3);
}

#[kani::proof]
#[kani::stub(recursive, g)]
fn recursive_target() {
    assert_eq!(recursive(8), 10);
}

#[kani::requires(x != y)]
#[kani::ensures(|result| *result == x)]
fn generic<T: Copy + PartialEq>(x: T, y: T) -> T {
    x
}

fn generic_replacement<T: Copy + PartialEq>(_x: T, y: T) -> T {
    y
}

#[kani::proof]
#[kani::stub(generic, generic_replacement)]
fn generic_instances() {
    assert_eq!(generic(1u8, 2u8), 2);
    assert_eq!(generic(3u16, 4u16), 4);
    assert_eq!(generic(5u8, 5u8), 5);
}

fn uncontracted(x: u8) -> u8 {
    x
}

#[kani::proof]
#[kani::stub(uncontracted, g)]
fn uncontracted_target() {
    assert_eq!(uncontracted(8), 10);
    assert_eq!(f(7), 8);
}

#[kani::proof]
#[kani::stub(f, requires_only)]
#[kani::stub(requires_only, g)]
fn transitive_replacement() {
    assert_eq!(f(8), 10);
    assert_eq!(requires_only(8), 10);
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
fn replacement_contract() {
    assert_eq!(f(8), 11);
    assert_eq!(contracted_replacement(8), 11);
}

#[kani::proof]
#[kani::stub(f, contracted_replacement)]
#[kani::stub_verified(contracted_replacement)]
fn replacement_verified_contract() {
    assert_eq!(f(8), 11);
}

#[kani::proof_for_contract(contracted_replacement)]
#[kani::stub(f, contracted_replacement)]
fn replacement_contract_target() {
    // Reach the exact selected provider in this harness, as well as its copied body.
    contracted_replacement(0);
    f(kani::any());
}
