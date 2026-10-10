// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a measure that Kani cannot evaluate at each iteration does not pass the decreases
//! check without being checked, and that Kani warns about it. The measures are wrong, except in
//! `checked_arithmetic_measure_harness`, whose check fails until
//! https://github.com/model-checking/kani/issues/4585 is fixed.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn if_measure_harness() {
    let flag: bool = kani::any();
    let mut i: u16 = 0;
    let mut up: u16 = 0;
    let mut down: u16 = 10;

    #[kani::loop_invariant(i <= 4 && up == i && down == 10 - i)]
    // Bug: `up` increases.
    #[kani::loop_decreases(if flag { up } else { down })]
    while i < 4 {
        up = up + 1;
        down = down - 1;
        i += 1;
    }
}

#[kani::proof]
fn call_measure_harness() {
    let mut i: u16 = 0;
    let mut up: u16 = 0;

    #[kani::loop_invariant(i <= 4 && up == i)]
    // Bug: `up` increases.
    #[kani::loop_decreases(up.wrapping_add(0))]
    while i < 4 {
        up = up + 1;
        i += 1;
    }
}

#[kani::proof]
fn index_measure_harness() {
    let mut a = [5u8, 5];
    let mut i = 0usize;

    #[kani::loop_invariant(i < 2 && a[0] <= 5 && a[1] == 5)]
    // Bug: `a[1]` does not decrease. The index is copied to a temporary before the loop.
    #[kani::loop_decreases(a[i])]
    while a[0] > 0 {
        a[0] -= 1;
        i = 1;
    }
}

#[kani::proof]
fn checked_arithmetic_measure_harness() {
    let mut i: u16 = 0;

    #[kani::loop_invariant(i <= 4)]
    // The measure decreases, but `4 - i` is computed once, before the loop.
    #[kani::loop_decreases(4 - i)]
    while i < 4 {
        i += 1;
    }
}
