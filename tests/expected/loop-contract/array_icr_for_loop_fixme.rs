// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z mem-predicates -Z quantifiers

//! Check for-loop invariant with quantifier for array increasing.
//! Having SMT issue that need to be fix (see issue #4282)

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::requires(a.len() < 256)]
#[kani::requires(kani::forall!(|i in (0, a.len())| a[i] < i32::MAX))]
#[kani::ensures(kani::forall!(|i in (0, a.len())| a[i] = on_entry(a[i]) + 1))]
fn array_inc(a: &mut [i32]) {
    let b: [u8; 60] = a.clone();
    #[kani::loop_invariant(i < 60
            && kani::forall!(|j in (kani::index, 60)| b[j] == a[j])
            && kani::forall!(|j in (0, kani::index)| a[j] == b[j] + 1)
    )]
    for i in 0..a.len() {
        a[i as usize] = a[i as usize] + 1;
    }
}

#[kani::requires(a.len() < 256)]
#[kani::requires(kani::forall!(|i in (0, a.len())| a[i] < i32::MAX))]
#[kani::ensures(kani::forall!(|i in (0, a.len())| a[i] = on_entry(a[i]) + 1))]
fn array_inc_iter_mut(a: &mut [i32]) {
    let b: [u8; 60] = a.clone();
    #[kani::loop_invariant(i < 60
            && kani::forall!(|j in (kani::index, 60)| b[j] == a[j])
            && kani::forall!(|j in (0, kani::index)| a[j] == b[j] + 1)
    )]
    for x in a.iter_mut() {
        *x = *x + 1;
    }
}

#[kani::proof_for_contract(array_inc)]
#[kani::solver(z3)]
fn check_array_inc(a: &mut [i32]) {
    let a: [i32; 256] = kani::any();
    let slice = kani::slice::any_slice_of_array(&a);
    array_inc(slice);
}

#[kani::proof_for_contract(array_inc_iter_mut)]
#[kani::solver(z3)]
fn check_array_inc_iter_mut(a: &mut [i32]) {
    let a: [i32; 256] = kani::any();
    let slice = kani::slice::any_slice_of_array(&a);
    array_inc_iter_mut(slice);
}
