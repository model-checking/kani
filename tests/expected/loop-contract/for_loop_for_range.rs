// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check for-loop invariant for Range.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    #[kani::loop_invariant( sum <= (kaniindex as u32 * 10 as u32) )]
    for i in 1..11 {
        sum = sum + (i as u32);
    }
    assert!(sum <= 100);
}
