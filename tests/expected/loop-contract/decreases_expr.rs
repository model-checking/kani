// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check decreases clause with a simple variable measure.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn decreases_expr_harness() {
    let mut x: u8 = kani::any_where(|i| *i >= 1 && *i <= 10);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        x -= 1;
    }

    assert!(x == 1);
}
