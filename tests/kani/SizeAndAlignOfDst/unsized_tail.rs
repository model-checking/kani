// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure we compute the size correctly including padding.

extern crate kani;

use kani::mem::{checked_align_of_raw, checked_size_of_raw};
use std::fmt::Debug;

#[derive(kani::Arbitrary)]
struct Pair<T, U: ?Sized>(T, U);

#[kani::proof]
fn check_adjusted_size_slice() {
    let tup: Pair<[u8; 5], [u16; 3]> = kani::any();
    let size = std::mem::size_of_val(&tup);

    let unsized_tup: *const Pair<[u8; 5], [u16]> = &tup as *const _ as *const _;
    let adjusted_size = std::mem::size_of_val(unsafe { &*unsized_tup });

    assert_eq!(size, adjusted_size);
}

#[kani::proof]
fn check_adjusted_size_dyn() {
    const EXPECTED_SIZE: usize = size_of::<Pair<u32, [u8; 5]>>();
    let tup: Pair<u32, [u8; 5]> = kani::any();
    let size = std::mem::size_of_val(&tup);
    assert_eq!(size, EXPECTED_SIZE);

    let unsized_tup: *const Pair<u32, dyn Debug> = &tup as *const _ as *const _;
    let adjusted_size = std::mem::size_of_val(unsafe { &*unsized_tup });
    assert_eq!(adjusted_size, EXPECTED_SIZE);
}

#[kani::proof]
pub fn checked_size_of_slice_is_zero() {
    let size_sized = checked_size_of_raw(&Pair((), ()));
    let size_slice = checked_size_of_raw(&Pair((), [(); 2]) as &Pair<(), [()]>);
    assert_eq!(size_sized, Some(0));
    assert_eq!(size_slice, Some(0));
}

#[kani::proof]
pub fn checked_size_of_slice_is_non_zero() {
    let size_sized = checked_size_of_raw(&Pair(0u8, 19i32));
    assert_eq!(size_sized, Some(8));

    let size_slice = checked_size_of_raw(&Pair(10u8, [1i32; 10]) as &Pair<u8, [i32]>);
    assert_eq!(size_slice, Some(44));
}

#[kani::proof]
pub fn checked_size_with_overflow() {
    let original = Pair(0u8, [(); usize::MAX]);
    let slice = &original as *const _ as *const Pair<u8, [()]>;
    assert_eq!(checked_size_of_raw(slice), Some(1));

    let invalid = slice as *const Pair<u8, [u8]>;
    assert_eq!(checked_size_of_raw(invalid), None);
}

#[kani::proof]
pub fn checked_align_of_dyn_tail() {
    let align_sized = checked_align_of_raw(&Pair(0u8, 19i32));
    assert_eq!(align_sized, Some(4));

    let align_dyn = checked_align_of_raw(&Pair(10u8, [1i32; 10]) as &Pair<u8, dyn Debug>);
    assert_eq!(align_dyn, Some(4));
}
