// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates

//! Check for-loop invariant for Vec.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let v: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    #[kani::loop_invariant( sum <= (kani_index as u32 * 10) )]
    for j in v {
        sum = sum + (j as u32);
    }
    assert!(sum <= 100);
}
