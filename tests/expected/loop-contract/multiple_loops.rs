// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts --enable-unstable --cbmc-args --object-bits 8

//! Check if loop contracts is correctly applied.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn multiple_loops() {
    let mut x: u8 = kani::any_where(|i| *i >= 10);

    if x != 20 {
        #[kani::loop_invariant(x >= 5)]
        while x > 5 {
            x = x - 1;
        }
    }

    assert!(x == 5 || x == 20);

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

/// Check that `loop-contracts` works correctly for harness
/// without loop contracts.
#[kani::proof]
fn no_loop_harness() {
    let x = 2;
    assert!(x == 2);
}

#[kani::proof]
fn multiple_loops_harness() {
    multiple_loops();
    simple_while_loops();
}
