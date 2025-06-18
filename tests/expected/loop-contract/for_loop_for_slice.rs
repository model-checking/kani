// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates

//! Check if for-loop invariant is correctly applied for slice.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    let s = kani::slice::any_slice_of_array(&a);
    let len = s.len();
    #[kani::loop_invariant( sum <= (kaniindex as u32 * u8::MAX as u32) )]
    for j in s {
        sum = sum + (j as u32);
    }
    assert!(sum <= (255 as u32 * len as u32));
}
