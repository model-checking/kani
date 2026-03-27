// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a non-decreasing measure correctly fails verification.
//! The loop body does not modify x, so the decreases clause cannot be proved.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn non_decreasing_harness() {
    let mut x: u8 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        // Bug: x is not modified, so it never decreases.
        // The decreases clause should fail.
    }
}
