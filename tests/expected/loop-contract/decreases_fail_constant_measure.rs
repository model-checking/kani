// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a constant measure, which cannot decrease, fails the decreases check.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

const MEASURE: u16 = 5;

#[kani::proof]
fn literal_measure_harness() {
    let mut x: u16 = kani::any_where(|i| *i >= 1 && *i <= 100);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(7)]
    while x > 1 {
        x = x - 1;
    }
}

#[kani::proof]
fn const_measure_harness() {
    let mut x: u16 = kani::any_where(|i| *i >= 1 && *i <= 100);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(MEASURE)]
    while x > 1 {
        x = x - 1;
    }
}
