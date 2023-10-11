// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

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