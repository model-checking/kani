// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if loop contracts is correctly applied.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
#[kani::solver(kissat)]
fn simple_while_loop_harness() {
    // Needed to avoid having `free` be removed as unused function. This is
    // because DFCC contract enforcement assumes that a definition for `free`
    // exists.
    let _ = Box::new(10);
    let mut x: u8 = kani::any_where(|i| *i >= 2);

    #[kani::loop_invariant(x >= 2)]
    while x > 2 {
        x = x - 1;
    }

    assert!(x == 2);
}
