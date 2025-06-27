// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags:

//! Check if the harness can be proved when loop contracts is not enabled.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn simple_while_loop_harness() {
    let mut x: u8 = 10;

    #[kani::loop_invariant(x >= 2)]
    while x > 2 {
        x = x - 1;
    }

    assert!(x == 2);
}
