// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that Kani handle different sets of stubbing correctly.
// I.e., not correctly replacing the stubs will cause a harness to fail.

fn identity(i: i8) -> i8 {
    i
}

fn decrement(i: i8) -> i8 {
    i.wrapping_sub(1)
}

fn increment(i: i8) -> i8 {
    i.wrapping_add(1)
}

#[kani::proof]
fn check_identity() {
    let n = kani::any();
    assert_eq!(identity(n), n);
}

#[kani::proof]
fn check_decrement() {
    let n = kani::any();
    kani::assume(n > i8::MIN);
    assert_eq!(decrement(n), n - 1);
}

#[kani::proof]
#[kani::stub(decrement, increment)]
fn check_decrement_is_increment() {
    let n = kani::any();
    kani::assume(n < i8::MAX);
    assert_eq!(decrement(n), n + 1);
}

#[kani::proof]
#[kani::stub(increment, identity)]
#[kani::stub(decrement, identity)]
fn check_all_identity() {
    let n = kani::any();
    assert_eq!(decrement(n), increment(n));
}

#[kani::proof]
#[kani::stub(decrement, identity)]
#[kani::stub(increment, identity)]
fn check_all_identity_2() {
    let n = kani::any();
    assert_eq!(decrement(n), n);
    assert_eq!(increment(n), n);
}

#[kani::proof]
#[kani::stub(decrement, increment)]
#[kani::stub(increment, identity)]
fn check_indirect_all_identity() {
    let n = kani::any();
    assert_eq!(decrement(n), n);
    assert_eq!(increment(n), n);
}
