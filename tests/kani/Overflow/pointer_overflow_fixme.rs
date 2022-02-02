// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail
// This should fail, but doesn't due to https://github.com/diffblue/cbmc/issues/6631

pub fn main() {
    let a = [0; 5];
    let i: i32 = 0;
    let ptr1: *const i32 = &a[1];
    let ptr2: *const i32 = &i;
    let ptr2 = unsafe { ptr2.offset(1) };
    let ptr_overflow1 = unsafe { ptr1.offset(isize::MAX) };
    let ptr_overflow2 = unsafe { ptr2.offset(isize::MAX) };
}
