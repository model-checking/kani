// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test relation comparison for thin pointers and fat pointers that have the same provenance.
//! Fat pointer comparisons take into consideration the data portion first,
//! if the comparison is not decisive, it will compare the size.
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
fn compare_equal<T: ?Sized>(obj1: *const T, obj2: *const T) {
    // Check Ord::cmp().
    assert_eq!(obj1.cmp(&obj2), Ordering::Equal);

    // Check relation operations that should return true.
    assert!(obj1 <= obj2);
    assert!(obj1 >= obj2);
    assert!(obj1 == obj2);

    // Check relation operations that should return false.
    assert!(!(obj1 > obj2));
    assert!(!(obj1 < obj2));
    assert!(!(obj1 != obj2));

    // Check Ord::{max, min}.
    assert_eq!(obj1.min(obj2), obj1);
    assert_eq!(obj1.max(obj2), obj1);
}

/// Check clamp operation.
fn check_clamp<T: ?Sized>(object: *const T, smaller: *const T, bigger: *const T) {
    assert_eq!(object.clamp(smaller, bigger), object);
    assert_eq!(object.clamp(smaller, object), object);
    assert_eq!(object.clamp(object, bigger), object);

    assert_eq!(object.clamp(bigger, bigger), bigger);
    assert_eq!(object.clamp(smaller, smaller), smaller);
}

/// Check comparison of thin pointers.
#[cfg_attr(kani, kani::proof)]
fn check_thin_ptr() {
    let array = [0u8; 10];
    let first_ptr: *const u8 = &array[0];
    let second_ptr: *const u8 = &array[5];

    compare_diff(first_ptr, second_ptr);
    compare_equal(first_ptr, first_ptr);
    check_clamp(&array[5], &array[0], &array[9]);
}

/// Check comparisons when slice size is the same but data pointer is different.
#[cfg_attr(kani, kani::proof)]
fn check_slice_data_ptr() {
    let array = [[0u8, 2]; 10];
    let first_ptr: *const [u8] = &array[0];
    let second_ptr: *const [u8] = &array[5];

    compare_diff(first_ptr, second_ptr);
    compare_equal(first_ptr, first_ptr);
    check_clamp::<[u8]>(&array[5], &array[0], &array[9]);
}

/// Check comparisons when slice size is different but the pointer is the same.
#[cfg_attr(kani, kani::proof)]
fn check_slice_len() {
    let array = [0u8; 10];
    let first_ptr: *const [u8] = &array[0..2];
    let second_ptr: *const [u8] = &array[0..4];

    compare_diff(first_ptr, second_ptr);
    compare_equal(first_ptr, first_ptr);
    check_clamp::<[u8]>(&array[4..6], &array[4..5], &array[4..]);
}

// Check comparison of box.
#[cfg_attr(kani, kani::proof)]
fn check_box_comparison() {
    let obj = Box::new([0u16, 10]);
    let first: *const [u16] = &obj[1..2];
    let second: *const [u16] = &obj[1..2];

    // Data address should be the same.
    assert_eq!(second as *const (), first as *const (), "Expected same data address");
    compare_equal(first, second);
}

// Allow us to run using rustc.
#[allow(dead_code)]
fn main() {
    check_thin_ptr();
    check_slice_data_ptr();
    check_box_comparison();
}
