// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant for when pat is a tuple.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn sum_pair((x, y): (u8, u8)) -> u32 {
    x as u32 + y as u32
}

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [(u8, u8); 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| sum_pair(a[i]) <= 10));
    #[kani::loop_invariant( sum <= (kani::index as u32 * 10 as u32) )]
    for (i, j) in a {
        sum = sum + (i as u32) + (j as u32);
    }
    assert!(sum <= 100);
}
