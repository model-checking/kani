// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a stale measure (one that doesn't change) fails verification.
//! The loop body modifies x but the decreases clause uses a different
//! variable y that is never modified, so the measure is stale.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn stale_measure_harness() {
    let mut x: u16 = kani::any_where(|i| *i >= 2 && *i <= 100);
    let y: u16 = x;

    #[kani::loop_invariant(x >= 1 && x <= y)]
    #[kani::loop_decreases(y)]
    while x > 1 {
        // x decreases, but the measure (y) never changes.
        x = x - 1;
    }

    // `y` keeps its value.
    assert!(x == 1 && y >= 2);
}
