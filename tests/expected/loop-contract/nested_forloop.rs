// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant for simple nested for-loop.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 20));
    let b: [u8; 20] = kani::any();
    kani::assume(kani::forall!(|i in (0,20)| b[i] <= 30));
    #[kani::loop_invariant( sum <= (kani::index as u32 * 620) )]
    for i in a.iter() {
        #[kani::loop_invariant( sum <= on_entry(sum) + (kani::index as u32 * 30) )]
        for j in b.iter() {
            sum = sum + (*j as u32);
        }
        sum = sum + (*i as u32)
    }
    assert!(sum <= 6200);
}
