// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a complete `#[kani::loop_modifies]` clause verifies on `while` loops whose body
//! declares variables that the loop contract transformation initializes at the loop head, and
//! on loops whose invariant uses `prev`. These variables only exist for the loop, so the user
//! cannot list them; they used to fail the assigns check (`Check that var_15 is assignable`,
//! `Check that __kani_prev_var_.. is assignable`).
//! The same loop also appears in a generic function instantiated twice and in a closure.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn call_in_body() {
    let a: [u8; 4] = kani::any();
    let mut i: usize = 0;
    let mut sum: u32 = 0;
    #[kani::loop_invariant(i <= 4)]
    #[kani::loop_modifies(&i, &sum)]
    while i < 4 {
        let x = unsafe { a.get_unchecked(i) };
        sum = sum.wrapping_add(*x as u32);
        i += 1;
    }
    assert!(i == 4);
}

#[kani::proof]
fn with_prev() {
    let mut i: u32 = 0;
    #[kani::loop_invariant(i <= 10 && prev(i) < i)]
    #[kani::loop_modifies(&i)]
    while i < 10 {
        i += 1;
    }
    assert!(i == 10);
}

fn sum_unchecked<T: Copy + Into<u32>>(a: [T; 4]) -> u32 {
    let mut i: usize = 0;
    let mut sum: u32 = 0;
    #[kani::loop_invariant(i <= 4)]
    #[kani::loop_modifies(&i, &sum)]
    while i < 4 {
        let x = unsafe { a.get_unchecked(i) };
        sum = sum.wrapping_add((*x).into());
        i += 1;
    }
    sum
}

#[kani::proof]
fn generic_function() {
    let a: [u8; 4] = kani::any();
    let b: [u16; 4] = kani::any();
    let _ = sum_unchecked(a);
    let _ = sum_unchecked(b);
}

#[kani::proof]
fn loop_in_closure() {
    let a: [u8; 4] = kani::any();
    let sum_of_a = || {
        let mut i: usize = 0;
        let mut sum: u32 = 0;
        #[kani::loop_invariant(i <= 4)]
        #[kani::loop_modifies(&i, &sum)]
        while i < 4 {
            let x = unsafe { a.get_unchecked(i) };
            sum = sum.wrapping_add(*x as u32);
            i += 1;
        }
        sum
    };
    let _ = sum_of_a();
}
