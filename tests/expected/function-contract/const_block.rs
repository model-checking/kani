// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! Checking that constant blocks are correctly verified in contracts.
//! See https://github.com/model-checking/kani/issues/3905

#![feature(ptr_alignment_type)]

use std::mem;
use std::ptr::{Alignment, NonNull};

#[derive(PartialEq)]
enum Enum {
    First,
    Second,
}

#[kani::ensures(|result| *result == Enum::First)]
const fn first() -> Enum {
    const { Enum::First }
}

#[kani::ensures(|result| *result == Enum::Second)]
const fn second() -> Enum {
    Enum::Second
}

#[kani::proof_for_contract(first)]
pub fn check_first() {
    let _ = first();
}

#[kani::proof_for_contract(second)]
pub fn check_second() {
    let _ = second();
}

#[kani::ensures(|result| result.as_usize().is_power_of_two())]
pub const fn Align_of<T>() -> Alignment {
    // This can't actually panic since type alignment is always a power of two.
    const { Alignment::new(mem::align_of::<T>()).unwrap() }
}

#[kani::ensures(|result| result.as_usize().is_power_of_two())]
pub const fn Align_of_no_const<T>() -> Alignment {
    // This can't actually panic since type alignment is always a power of two.
    Alignment::new(mem::align_of::<T>()).unwrap()
}

#[kani::proof_for_contract(Align_of)]
pub fn check_of_i32() {
    let _ = Align_of::<i32>();
}

#[kani::proof_for_contract(Align_of_no_const)]
pub fn check_of_i32_no_const() {
    let _ = Align_of_no_const::<i32>();
}

#[kani::requires(true)]
pub const unsafe fn byte_add_n<T>(s: NonNull<T>, count: usize) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(s.as_ptr().byte_add(count)) }
}

#[kani::proof_for_contract(byte_add_n)]
pub fn non_null_byte_add_dangling_proof() {
    let ptr = NonNull::<i32>::dangling();
    assert!(ptr.as_ptr().addr() == 4);
    assert!(ptr.as_ptr().addr() <= (isize::MAX as usize));
    unsafe {
        let _ = byte_add_n(ptr, 0);
    }
}
