// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test relation comparison of fat pointers to slices.
use std::cmp::*;

/// Check comparison operators for two different elements.
fn compare<T: ?Sized>(smaller: *const T, bigger: *const T) {
    // Check relation operations that should return true.
    assert!(smaller < bigger);
    assert!(smaller <= bigger);
    assert!(bigger > smaller);
    assert!(bigger >= smaller);
    assert!(bigger != smaller);
    assert!(!(bigger == smaller));
}

#[kani::proof]
fn check_slice_ptr() {
    let array = [[0u8, 2]; 10];
    let first_ptr: *const [u8] = &array[0];
    let second_ptr: *const [u8] = &array[5];

    compare(first_ptr, second_ptr);
}

trait Dummy {}
impl Dummy for u8 {}

#[kani::proof]
fn check_dyn_ptr() {
    let array = [0u8; 10];
    let first_ptr: *const dyn Dummy = &array[0];
    let second_ptr: *const dyn Dummy = &array[5];

    compare(first_ptr, second_ptr);
}
