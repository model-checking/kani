// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `ptr_offset_from` fails if the object that the arguments
// point to are not the same.
#![feature(core_intrinsics)]
use std::intrinsics::ptr_offset_from;

#[kani::proof]
fn main() {
    let a = [0; 5];
    let b = [0; 5];
    let ptr1: *const i32 = &a[1];
    let ptr2: *const i32 = &b[3];
    unsafe {
        let offset = ptr_offset_from(ptr2, ptr1);
    }
}
