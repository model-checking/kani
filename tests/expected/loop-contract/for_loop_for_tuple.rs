// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates

//! Check for-loop invariant for when pat is a tuple.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [(u8, u8); 10] = kani::any();

    #[kani::loop_invariant( sum <= (kani_index as u32 * u8::MAX as u32 * 2) )]
    for (i, j) in a {
        sum = sum + (i as u32) + (j as u32);
    }
    assert!(sum <= 20 * u8::MAX as u32);
}
