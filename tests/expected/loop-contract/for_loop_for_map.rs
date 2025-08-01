// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant for Map.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 10));
    #[kani::loop_invariant( sum <= (kani_index as u32 * 11) )]
    for i in a.iter().map(|x| x + 1) {
        sum = sum + (i as u32);
    }
    assert!(sum <= 110);
}
