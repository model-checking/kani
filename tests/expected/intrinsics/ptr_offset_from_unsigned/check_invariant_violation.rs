// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(core_intrinsics)]
use std::intrinsics::ptr_offset_from_unsigned;

#[kani::proof]
fn check_failure() {
    let a = [0; 5];
    let ptr0: *const i32 = &a[0];
    let ptr1: *const i32 = &a[1];
    unsafe {
        let _distance = ptr_offset_from_unsigned(ptr0, ptr1);
    }
}
