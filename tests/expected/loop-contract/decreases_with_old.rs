// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check decreases clause combined with on_entry (old) values.
//! Inspired by Verus test_variables_not_havoc_basic.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn decreases_with_old_harness() {
    let mut x: u8 = kani::any_where(|i| *i >= 1 && *i <= 20);
    let y: u8 = 42;

    #[kani::loop_invariant(x >= 1 && on_entry(x) >= 1)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        x -= 1;
    }

    assert!(x == 1);
    // y should not be havocked
    assert!(y == 42);
}
