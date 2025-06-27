// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if loop-loop invariant is correctly applied.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn main() {
    let mut i = 100;
    #[kani::loop_invariant(i<=100 && i >= 4 && i % 2 ==0)]
    loop {
        i = i - 2;
        if i == 2 {
            break;
        }
    }
    assert!(i == 2);
}
