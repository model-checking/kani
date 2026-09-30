// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! FIXME: Nested loops with decreases clauses fail because the inner loop's
//! assigns clause checking conflicts with the outer loop's instrumentation.
//! Tracked in: https://github.com/model-checking/kani/issues/3168

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn fixme_nested_loops_decreases_harness() {
    let mut i: u8 = 10;
    let mut j: u8;

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
}
