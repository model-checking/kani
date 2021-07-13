// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-overflow-checks

// We use `--no-overflow-checks` in this test to avoid getting
// a verification failure:
// [overflow.2] arithmetic overflow on signed - in var_23 - var_24: FAILURE
// [overflow.4] arithmetic overflow on signed - in var_33 - var_34: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

#![feature(core_intrinsics)]
use std::intrinsics::ptr_offset_from;

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
