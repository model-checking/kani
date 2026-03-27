// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a constant decreases expression fails verification.
//! A constant never strictly decreases, so termination cannot be proved.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn constant_decreases_harness() {
    let mut x: u8 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(42u8)]
    while x > 1 {
        x = x - 1;
    }
}
