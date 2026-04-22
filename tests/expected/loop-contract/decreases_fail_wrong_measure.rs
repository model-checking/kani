// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a wrong decreases expression fails verification.
//! The measure increases instead of decreasing.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn wrong_decreases_harness() {
    let mut x: u16 = kani::any_where(|i| *i >= 2 && *i <= 200);

    #[kani::loop_invariant(x >= 2)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        // Bug: x increases instead of decreasing.
        // Using u16 to avoid overflow obscuring the decreases failure.
        x = x + 1;
    }
}
