// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: -Zmir-opt-level=2

// kani-flags: -Z loop-contracts

//! Check that the decreases clause is checked on the measure that the user wrote when MIR
//! optimizations (e.g. copy propagation and dead store elimination) are enabled.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn variable_measure_harness() {
    let mut x: u16 = kani::any_where(|i| *i >= 1 && *i <= 100);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        x = x - 1;
    }
}

#[kani::proof]
fn stale_measure_harness() {
    let mut x: u16 = kani::any_where(|i| *i >= 2 && *i <= 100);
    let y: u16 = x;

    #[kani::loop_invariant(x >= 1 && x <= y)]
    // Bug: `y` never changes.
    #[kani::loop_decreases(y)]
    while x > 1 {
        x = x - 1;
    }

    assert!(x == 1 && y >= 2);
}

#[inline(never)]
fn count_up_to(n: u32) -> u32 {
    let mut i: u32 = 0;
    #[kani::loop_invariant(i <= n)]
    // Bug: `n` never changes.
    #[kani::loop_decreases(n)]
    while i < n {
        i += 1;
    }
    i
}

#[kani::proof]
fn parameter_measure_harness() {
    let n: u32 = kani::any_where(|i| *i <= 5);
    assert!(count_up_to(n) == n);
}
