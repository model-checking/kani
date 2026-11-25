// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! Panics that the `panics_only_if` condition does not permit, undefined behavior on the way to
//! a permitted panic, wrong postconditions and panics in the harness itself are all reported.

#[kani::panics_only_if(x == 0)]
fn unexpected_panic(x: u8) -> u8 {
    if x == 7 {
        panic!("seven");
    }
    100 / x
}

#[kani::proof_for_contract(unexpected_panic)]
fn check_unexpected_panic() {
    unexpected_panic(kani::any());
}

#[kani::panics_only_if(i >= 4)]
fn ub_before_panic(i: usize) -> u8 {
    let array = [1u8; 4];
    if i == 9 {
        // Undefined behavior on an input for which the function then panics.
        let _ = unsafe { *array.as_ptr().add(i) };
    }
    array[i]
}

#[kani::proof_for_contract(ub_before_panic)]
fn check_ub_before_panic() {
    ub_before_panic(kani::any());
}

#[kani::panics_only_if(x > 200)]
#[kani::ensures(|result: &u8| *result == x)]
fn wrong_postcondition(x: u8) -> u8 {
    if x > 200 {
        panic!("too big");
    }
    x.wrapping_add(1)
}

#[kani::proof_for_contract(wrong_postcondition)]
fn check_wrong_postcondition() {
    wrong_postcondition(kani::any());
}

#[kani::panics_only_if(true)]
fn identity(x: u8) -> u8 {
    x
}

#[kani::proof_for_contract(identity)]
fn check_harness_panics() {
    let x: u8 = kani::any();
    assert!(x != 3, "before the call");
    identity(x);
    assert!(x != 5, "after the call");
}
