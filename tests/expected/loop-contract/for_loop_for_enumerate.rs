// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z quantifiers -Z mem-predicates

//! Check for-loop invariant for Range.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 20));
    #[kani::loop_invariant( sum <= (kani::index as u32 * 29) )]
    for (i, j) in a.iter().enumerate() {
        sum = sum + (i as u32) + (*j as u32);
    }
    assert!(sum <= 290);
}
