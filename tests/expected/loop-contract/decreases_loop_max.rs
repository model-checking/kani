// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Loop max function with decreases clause.
//! Inspired by Prusti's loop_max.rs test.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn loop_max(x: u8, y: u8) -> u8 {
    let mut r = x;

    #[kani::loop_invariant(x <= r && r <= y)]
    #[kani::loop_decreases(y - r)]
    while r < y {
        r += 1;
    }

    r
}

#[kani::proof]
fn loop_max_harness() {
    let x: u8 = kani::any_where(|i| *i > 0 && *i < 100);
    let y: u8 = kani::any_where(|i| *i > 0 && *i < 100);
    kani::assume(x <= y);
    let result = loop_max(x, y);
    assert!(result >= x);
    assert!(result >= y);
}
