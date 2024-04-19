// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check whether Kani fails if users forgot to annotate recursive
//! functions with `#[kani::recursion]` when using function contracts.

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
