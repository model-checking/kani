// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant with quantifier over array.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 10));

    #[kani::loop_invariant( sum <= (kaniindex as u32 * 10) )]
    for j in a.iter() {
        sum = sum + (*j as u32);
    }
    assert!(sum <= 100);
}
