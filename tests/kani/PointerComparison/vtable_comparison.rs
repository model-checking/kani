// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test relation comparison for vtable comparisons.
//! Fat pointer comparison take into consideration their data address and their metadata.
//! For traits, the pointer comparison will take into consideration the vtable address.
//!
//! Note that the vtable address is implementation dependent and Kani will pick a certain order.
//! The order should be consistent during one program execution, but it isn't stable.
//! The order might change with different versions of the compiler or different compilation
//! options.
use std::any::Any;
use std::cmp::*;

/// Check comparison operators for two different elements.
fn compare_diff<T: ?Sized>(smaller: *const T, bigger: *const T) {
    assert!(bigger != smaller);
    assert!(!(bigger == smaller));
    assert!(!(std::ptr::eq(bigger, smaller)));

    // Check Ord::cmp().
    assert_eq!(smaller.cmp(&bigger), Ordering::Less);
    assert_eq!(bigger.cmp(&smaller), Ordering::Greater);

    // Check relation operations that should return true.
    assert!(smaller < bigger);
    assert!(smaller <= bigger);
    assert!(bigger > smaller);
    assert!(bigger >= smaller);

    // Check relation operations that should return false.
    assert!(!(smaller > bigger));
    assert!(!(smaller >= bigger));
    assert!(!(bigger <= smaller));
    assert!(!(bigger < smaller));

    // Check Ord::{max, min}.
    assert_eq!(smaller.min(bigger), smaller);
    assert_eq!(smaller.max(bigger), bigger);
    assert_eq!(bigger.min(smaller), smaller);
    assert_eq!(bigger.max(smaller), bigger);
}

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(5))]
fn check_union_dyn_ptr() {
    union MyUnion {
        num: u8,
        character: char,
        empty: (),
    }

    let my_union = MyUnion { character: 'a' };
    let my_char: *const dyn Any = unsafe { &my_union.character };
    let my_num: *const dyn Any = unsafe { &my_union.num };
    let my_empty: *const dyn Any = unsafe { &my_union.empty };

    // Data address should be the same.
    assert_eq!(my_char as *const (), my_num as *const (), "Expected same data address");
    assert_eq!(my_char as *const (), my_empty as *const (), "Expected same data address");

    // Order between executions is unstable. Sort them before checking the operations.
    let mut my_vec = vec![my_char, my_num, my_empty];
    my_vec.sort();

    compare_diff(my_vec[0], my_vec[1]);
}

// Check that a pointer to a transparent struct is different than it's element.
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(4))]
fn check_transparent_dyn_ptr() {
    #[repr(transparent)]
    struct Trans(u64);

    let obj = Trans(10);
    let inner: *const dyn Any = &obj.0;
    let outer: *const dyn Any = &obj;

    // Data address should be the same.
    assert_eq!(outer as *const (), inner as *const (), "Expected same data address");

    // Order between executions is unstable. Sort them before checking the operations.
    let mut my_vec = vec![inner, outer];
    my_vec.sort();

    compare_diff(my_vec[0], my_vec[1]);
}

// Check that a pointer to a struct is different than it's first element.
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(4))]
fn check_struct_first_ptr() {
    struct Container {
        first: usize,
        #[allow(dead_code)]
        second: isize,
    }

    let obj = Container { first: 10, second: 100 };
    let inner: *const dyn Any = &obj.first;
    let outer: *const dyn Any = &obj;

    // Data address should be the same.
    assert_eq!(outer as *const (), inner as *const (), "Expected same data address");

    // Order between executions is unstable. Sort them before checking the operations.
    let mut my_vec = vec![inner, outer];
    my_vec.sort();

    compare_diff(my_vec[0], my_vec[1]);
}

// Check that comparing pointers to subsequent elements of a struct works when the first element
// size is zero (ZST).
#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(4))]
fn check_struct_elements_ptr() {
    struct Container {
        first: (),
        second: isize,
    }

    let obj = Container { first: (), second: 100 };
    let first: *const dyn Any = &obj.first;
    let second: *const dyn Any = &obj.second;

    // Data address should be the same.
    assert_eq!(second as *const (), first as *const (), "Expected same data address");

    // Order between executions is unstable. Sort them before checking the operations.
    let mut my_vec = vec![first, second];
    my_vec.sort();

    compare_diff(my_vec[0], my_vec[1]);
}

// Allow us to run using rustc.
#[allow(dead_code)]
fn main() {
    check_union_dyn_ptr();
    check_transparent_dyn_ptr();
    check_struct_first_ptr();
    check_struct_elements_ptr();
}
