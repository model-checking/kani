// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! The test cases try to ensure applying the hypothesis is only done in
//! inductive verification of a function call. E.g. on every second encounter of
//! the called function in its own call stack rather than globally in the
//! program.
//!
//! In each case we have a recursive function that is called and we expect that
//! the recursive call within the first call has the hypothesis applied. (That's
//! not actually tested here but separately.)
//!
//! Then we call the function again and we've set up the cases such that *if*
//! the actual body is used then that call with fail (once because of panic,
//! once because the postcondition is violated). If instead the hypothesis (e.g.
//! contract replacement) is used we'd expect the verification to succeed.

#[kani::ensures(result < 3)]
fn fail_on_two(i: i32) -> i32 {
    match i {
        0 => fail_on_two(i + 1),
        1 => 2,
        _ => unreachable!("fail on two"),
    }
}

#[kani::proof_for_contract(fail_on_two)]
fn harness() {
    let first = fail_on_two(0);
    let _ = fail_on_two(first);
}

#[kani::ensures(result < 3)]
fn fail_on_two_in_postcondition(i: i32) -> i32 {
    let j = i + 1;
    if i < 2 {
        fail_on_two_in_postcondition(j)
    } else {
        j
    }
}

#[kani::proof_for_contract(fail_on_two_in_postcondition)]
fn harness2() {
    let first = fail_on_two_in_postcondition(1);
    let _ = fail_on_two_in_postcondition(first);
}