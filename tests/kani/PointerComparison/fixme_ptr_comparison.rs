// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test relation comparison of fat pointers to slices.
use std::cmp::*;

/// Check comparison operators for two different elements.
fn compare_diff<T: ?Sized>(smaller: *const T, bigger: *const T) {
    // Check Ord::cmp().
    assert_eq!(smaller.cmp(&bigger), Ordering::Less);
    assert_eq!(bigger.cmp(&smaller), Ordering::Greater);

    // Check relation operations that should return true.
    assert!(smaller < bigger);
    assert!(smaller <= bigger);
    assert!(bigger > smaller);
    assert!(bigger >= smaller);
    assert!(bigger != smaller);

    // Check relation operations that should return false.
    assert!(!(smaller > bigger));
    assert!(!(smaller >= bigger));
    assert!(!(bigger <= smaller));
    assert!(!(bigger < smaller));
    assert!(!(bigger == smaller));
    assert!(!(std::ptr::eq(bigger, smaller)));

    // Check Ord::{max, min}.
    assert_eq!(smaller.min(bigger), smaller);
    assert_eq!(smaller.max(bigger), bigger);
    assert_eq!(bigger.min(smaller), smaller);
    assert_eq!(bigger.max(smaller), bigger);
}

/// Check comparison operators for the same object.
fn compare_equal<T: ?Sized>(object: *const T) {
    // Check Ord::cmp().
    assert_eq!(object.cmp(&object), Ordering::Equal);

    // Check relation operations that should return true.
    assert!(object <= object);
    assert!(object >= object);
    assert!(object == object);

    // Check relation operations that should return false.
    assert!(!(object > object));
    assert!(!(object < object));
    assert!(!(object != object));

    // Check Ord::{max, min}.
    assert_eq!(object.min(object), object);
    assert_eq!(object.max(object), object);
}

/// Check clamp operation.
fn check_clamp<T: ?Sized>(object: *const T, smaller: *const T, bigger: *const T) {
    assert_eq!(object.clamp(smaller, bigger), object);
    assert_eq!(object.clamp(smaller, object), object);
    assert_eq!(object.clamp(object, bigger), object);

    assert_eq!(object.clamp(bigger, bigger), bigger);
    assert_eq!(object.clamp(smaller, smaller), smaller);
}

#[cfg_attr(kani, kani::proof)]
fn check_thin_ptr() {
    let array = [0u8; 10];
    let first_ptr: *const u8 = &array[0];
    let second_ptr: *const u8 = &array[5];

    compare_diff(first_ptr, second_ptr);
    compare_equal(first_ptr);
    check_clamp(&array[5], &array[0], &array[9]);
}

#[cfg_attr(kani, kani::proof)]
fn check_slice_ptr() {
    let array = [[0u8, 2]; 10];
    let first_ptr: *const [u8] = &array[0];
    let second_ptr: *const [u8] = &array[5];

    compare_diff(first_ptr, second_ptr);
    compare_equal(first_ptr);
    check_clamp::<[u8]>(&array[5], &array[0], &array[9]);
}

trait Dummy {}
impl Dummy for u8 {}

#[cfg_attr(kani, kani::proof)]
fn check_dyn_ptr() {
    let array = [0u8; 10];
    let first_ptr: *const dyn Dummy = &array[0];
    let second_ptr: *const dyn Dummy = &array[5];

    compare_diff(first_ptr, second_ptr);
    compare_equal(first_ptr);
    check_clamp::<dyn Dummy>(&array[5], &array[0], &array[9]);
}

// Allow us to run usign rustc.
fn main() {
    check_thin_ptr();
    check_slice_ptr();
}
