// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#[kani::proof]
fn main() {
    let a = [0; 5];
    let i: i32 = 0;
    let ptr1: *const i32 = &a[0];
    let ptr_overflow1 = unsafe { ptr1.offset(isize::MAX) };
}
