// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a non-decreasing measure correctly fails verification.
//! The loop body sets x to a nondeterministic value that satisfies the
//! invariant but does not guarantee strict decrease.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn non_decreasing_harness() {
    let mut x: u16 = kani::any_where(|i| *i >= 2 && *i <= 300);

    #[kani::loop_invariant(x >= 2)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        // Bug: x increases instead of decreasing.
        // The decreases clause should fail.
        x = x + 1;
    }
}
