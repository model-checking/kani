// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(core_intrinsics)]
use std::intrinsics::ptr_offset_from;

#[kani::proof]
fn main() {
    let a = [0; 5];
    let b = [0; 5];
    let ptr1: *const i32 = &a[1];
    let ptr2: *const i32 = &a[3];
    let ptr3: *const i64 = &b[1];
    let ptr4: *const i64 = &b[3];
    unsafe {
        assert!(ptr_offset_from(ptr2, ptr1) == 2);
        assert!(ptr_offset_from(ptr1, ptr2) == -2);
        assert!(ptr_offset_from(ptr4, ptr3) == 2);
        assert!(ptr_offset_from(ptr3, ptr4) == -2);
    }
}
