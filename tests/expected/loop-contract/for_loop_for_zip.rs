// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant for Vec.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn incr_mul(x: u8, y: u8) -> u32 {
    (x as u32 + 1) * (y as u32 + 1)
}

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    let b: [u8; 15] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| incr_mul (a[i], b[i]) <= 100));
    #[kani::loop_invariant( kaniindex <= 10 && sum <= (kaniindex as u32 * 99) )]
    for (i, j) in a.iter().zip(b.iter()) {
        sum = sum + (*i as u32) + (*j as u32);
    }
    assert!(sum <= 990);
}
