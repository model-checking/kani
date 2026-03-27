// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Loop max function with decreases clause.
//! Inspired by Prusti's loop_max.rs test.
//! Uses a countdown variable to avoid unsigned subtraction in the measure.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn loop_max(x: u8, y: u8) -> u8 {
    let mut r = x;
    let mut remaining: u8 = y - x;

    #[kani::loop_invariant(x <= r && r <= y && remaining == y - r)]
    #[kani::loop_decreases(remaining)]
    while r < y {
        r += 1;
        remaining -= 1;
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
