// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(core_intrinsics)]
use std::intrinsics::ptr_offset_from_unsigned;

#[kani::proof]
fn check_distance_i32() {
    let a = [0; 5];
    let ptr0: *const i32 = &a[0];
    let ptr1: *const i32 = &a[1];
    let ptr2: *const i32 = &a[2];
    unsafe {
        assert_eq!(ptr_offset_from_unsigned(ptr2, ptr0), 2);
        assert_eq!(ptr_offset_from_unsigned(ptr1, ptr0), 1);
        assert_eq!(ptr_offset_from_unsigned(ptr2, ptr2), 0);
    }
}

#[kani::proof]
fn check_distance_i64() {
    let a = [0i64; 5];
    let ptr0: *const i64 = &a[0];
    let ptr1: *const i64 = &a[1];
    let ptr2: *const i64 = &a[2];
    unsafe {
        assert_eq!(ptr_offset_from_unsigned(ptr2, ptr0), 2);
        assert_eq!(ptr_offset_from_unsigned(ptr1, ptr0), 1);
        assert_eq!(ptr_offset_from_unsigned(ptr1, ptr1), 0);
    }
}
