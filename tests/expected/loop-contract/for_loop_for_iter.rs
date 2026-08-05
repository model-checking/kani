// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates

//! Check if for-loop invariant is correctly applied for array.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    #[kani::loop_invariant( sum <= (kani::index as u32 * u8::MAX as u32) )]
    for j in a.iter() {
        sum = sum + (*j as u32);
    }
    assert!(sum <= 2550);
}
