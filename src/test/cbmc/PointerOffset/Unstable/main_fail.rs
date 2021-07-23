// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

#![feature(core_intrinsics)]
use std::intrinsics::ptr_offset_from;

fn main() {
    let a = [0; 5];
    let b = [0; 5];
    let ptr1: *const i32 = &a[1];
    let ptr2: *const i32 = &b[3];
    unsafe {
        // Offset operations result in Undefined Behavior if
        // some conditions are violated. More info at:
        // https://doc.rust-lang.org/std/primitive.pointer.html#method.offset_from
        // In particular, the below call to `ptr_offset_from` is expected
        // to fail because `ptr1` and `ptr2` point to different objects.
        let offset = ptr_offset_from(ptr2, ptr1);
    }
}
