// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check decreases clause on nested loops, each with their own decreases.
//! Inspired by CBMC and Verus nested loop termination tests.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn nested_loops_decreases_harness() {
    let mut i: u8 = 10;
    let mut j: u8 = 10;

    #[kani::loop_invariant(i <= 10)]
    #[kani::loop_decreases(i)]
    while i > 0 {
        j = 5;
        #[kani::loop_invariant(j <= 5)]
        #[kani::loop_decreases(j)]
        while j > 0 {
            j -= 1;
        }
        i -= 1;
    }

    assert!(i == 0);
    assert!(j == 0);
}
