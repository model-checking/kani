// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant for Chain.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

use std::cmp::min;

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let mut a: [u8; 10] = kani::any();
    let mut b: [u8; 5] = [5; 5];
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 10));
    #[kani::loop_invariant( sum <= min(kaniindex as u32, 10) * 10 + (kaniindex as u32).saturating_sub(10) * 5 )]
    for j in a.iter().chain(b.iter()) {
        sum = sum + (*j as u32);
    }
    assert!(sum <= 125);
}
