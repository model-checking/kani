// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags:

//! Check if loop contracts is correctly applied.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]


#[kani::proof]
fn main() {
    let mut x: u8 = kani::any_where(|i| *i >= 2);


    #[kani::loop_invariant(x >= 2)]
    while x > 2{
        x = x - 1;
    };


    assert!(x == 2);
}
