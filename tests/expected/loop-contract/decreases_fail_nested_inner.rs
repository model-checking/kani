// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a nested loop with a non-decreasing inner loop fails.
//! The outer loop correctly decreases, but the inner loop body
//! does not modify j, so its decreases clause fails.
//! Inspired by Verus loop_decreases2 test.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn nested_inner_fail_harness() {
    let mut i: u8 = 5;
    let mut j: u8 = 5;

    #[kani::loop_invariant(i <= 5)]
    #[kani::loop_decreases(i)]
    while i > 0 {
        j = 3;
        #[kani::loop_invariant(j <= 3)]
        #[kani::loop_decreases(j)]
        while j > 0 {
            // Bug: j is not modified in the inner loop.
        }
        i -= 1;
    }
}
