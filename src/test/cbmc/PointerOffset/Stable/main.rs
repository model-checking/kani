// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-overflow-checks

// We use `--no-overflow-checks` in this test to avoid getting
// a verification failure:
// [overflow.1] arithmetic overflow on signed - in var_11 - var_12: FAILURE
// [overflow.2] arithmetic overflow on signed - in var_11 - var_12: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

fn main() {
    // pub unsafe fn offset_from(self, origin: *const T) -> isize
    // Calculates the distance between two pointers. The returned value
    // is in units of T: the distance in bytes is divided by mem::size_of::<T>().
    // https://doc.rust-lang.org/src/core/ptr/const_ptr.rs.html#367-375
    let a = [0; 5];
    let b = [0; 5];
    let ptr1: *const i32 = &a[1];
    let ptr2: *const i32 = &a[3];
    let ptr3: *const i64 = &b[1];
    let ptr4: *const i64 = &b[3];
    unsafe {
        assert!(ptr2.offset_from(ptr1) == 2);
        assert!(ptr1.offset_from(ptr2) == -2);
        assert!(ptr4.offset_from(ptr3) == 2);
        assert!(ptr3.offset_from(ptr4) == -2);
    }
}
