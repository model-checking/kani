// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check the use of "old" in loop invariant

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
pub fn loop_with_old() {
    let mut x: u8 = kani::any_where(|v| *v < 10);
    let mut y: u8 = kani::any();
    let mut i = 0;
    #[kani::loop_invariant( (i<=5) && (x <= on_entry(x) + i) && (on_entry(x) + i == on_entry(i) + x))]
    while i < 5 {
        if i == 0 {
            y = x
        }
        x += 1;
        i += 1;
    }
}
