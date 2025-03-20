// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if loop contracts is correctly applied.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
pub fn loop_with_old_and_prev() {
    let mut i = 100;
    #[kani::loop_invariant((i >= 2) && (i <= 100) && (i % 2 == 0) && (old(i) == 100) && (prev(i) == i + 2))]
    while i > 2 {
        if i == 1 {
            break;
        }
        i = i - 2;
    }
    assert!(i == 2);
}
