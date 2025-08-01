// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant for nested for-loop for enumerate, which involves tuple pat.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 20));
    let b: [u8; 20] = kani::any();
    kani::assume(kani::forall!(|i in (0,20)| b[i] <= 30));
    #[kani::loop_invariant( i1 <= 10 && (i1 == kani_index || i1 + 1 == kani_index) && sum <= (kani_index as u32 * 620) )]
    for (i1, i2) in a.iter().enumerate() {
        #[kani::loop_invariant( j1 <= 20 && (j1 == kani_index || j1 + 1 == kani_index) && sum <= on_entry(sum) + (kani_index as u32 * 30) )]
        for (j1, j2) in b.iter().enumerate() {
            sum = sum + (*j2 as u32);
        }
        sum = sum + (*i2 as u32)
    }
    assert!(sum <= 6200);
}
