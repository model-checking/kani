// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts --enable-unstable --cbmc-args --object-bits 8

//! Check if loop contracts is correctly applied.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn multiple_loops() {
    let mut x: u8 = kani::any_where(|i| *i >= 10);

    #[kani::loop_invariant(x >= 5)]
    while x > 5 {
        x = x - 1;
    }

    assert!(x == 5);

    #[kani::loop_invariant(x >= 2)]
    while x > 2 {
        x = x - 1;
    }

    assert!(x == 2);
}

fn simple_while_loops() {
    let mut x: u8 = kani::any_where(|i| *i >= 10);
    let mut y: u8 = kani::any_where(|i| *i >= 10);

    #[kani::loop_invariant(x >= 2)]
    while x > 2 {
        x = x - 1;
        #[kani::loop_invariant(y >= 2)]
        while y > 2 {
            y = y - 1;
        }
    }

    assert!(x == 2);
}

#[kani::proof]
fn multiple_loops_harness() {
    // Needed to avoid having `free` be removed as unused function. This is
    // because DFCC contract enforcement assumes that a definition for `free`
    // exists.
    let _ = Box::new(10);
    multiple_loops();
    simple_while_loops();
}
