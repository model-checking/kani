// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure we compute the size correctly including padding.

extern crate kani;

use kani::mem::{checked_align_of_raw, checked_size_of_raw};
use std::fmt::Debug;
use std::mem;

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
pub fn checked_align_of_dyn_from_tail() {
    let concrete = Pair(0u8, 19i32);
    let dyn_ptr = &concrete as &Pair<u8, dyn Debug>;
    let expected = std::mem::align_of::<Pair<u8, i32>>();
    // Check methods with concrete.
    assert_eq!(checked_align_of_raw(&concrete), Some(expected));
    assert_eq!(std::mem::align_of_val(&concrete), expected);
    // Check methods with dynamic.
    assert_eq!(checked_align_of_raw(dyn_ptr), Some(expected));
    assert_eq!(std::mem::align_of_val(dyn_ptr), expected);
}

#[kani::proof]
pub fn checked_align_of_dyn_from_head() {
    let concrete = Pair(19i32, 10u8);
    let dyn_ptr = &concrete as &Pair<i32, dyn Debug>;
    let expected = std::mem::align_of::<Pair<i32, u8>>();
    // Check methods with concrete.
    assert_eq!(checked_align_of_raw(&concrete), Some(expected));
    assert_eq!(std::mem::align_of_val(&concrete), expected);
    // Check methods with dynamic.
    assert_eq!(checked_align_of_raw(dyn_ptr), Some(expected));
    assert_eq!(std::mem::align_of_val(dyn_ptr), expected);
}

#[kani::proof]
pub fn checked_align_of_slice_from_tail() {
    let concrete = Pair([0u8; 5], ['a'; 7]);
    let slice_ptr = &concrete as &Pair<[u8; 5], [char]>;
    let expected = std::mem::align_of::<Pair<[u8; 5], [char; 5]>>();
    // Check methods with concrete.
    assert_eq!(checked_align_of_raw(&concrete), Some(expected));
    assert_eq!(std::mem::align_of_val(&concrete), expected);
    // Check methods with dynamic.
    assert_eq!(checked_align_of_raw(slice_ptr), Some(expected));
    assert_eq!(std::mem::align_of_val(slice_ptr), expected);
}

#[kani::proof]
pub fn checked_align_of_slice_from_head() {
    let concrete = Pair(['a'; 7], [0u8; 5]);
    let slice_ptr = &concrete as &Pair<[char; 7], [u8]>;
    let expected = std::mem::align_of::<Pair<[char; 7], [u8; 5]>>();
    // Check methods with concrete.
    assert_eq!(checked_align_of_raw(&concrete), Some(expected));
    assert_eq!(std::mem::align_of_val(&concrete), expected);
    // Check methods with dynamic.
    assert_eq!(checked_align_of_raw(slice_ptr), Some(expected));
    assert_eq!(std::mem::align_of_val(slice_ptr), expected);
}
