// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant with quantifier for array increasing.
//! Having SMT issue that need to be fix (see issue #4282)

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
#[kani::solver(z3)]
fn array_inc() {
    let a: [u8; 60] = kani::any();
    kani::assume(kani::forall!(|i in (0,60)| a[i] <= u8::MAX));
    let mut b: [u8; 60] = a.clone();
    #[kani::loop_invariant(i < 60
            && kani::forall!(|j in (kani::index, 60)| b[j] == a[j])
            && kani::forall!(|j in (0, kani::index)| b[j] == a[j] + 1)
    )]
    for i in 0..60 {
        b[i as usize] = a[i as usize] + 1;
    }
}
