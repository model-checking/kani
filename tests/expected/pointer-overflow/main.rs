// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that overflows for pointer arithmetic are reported

#[kani::proof]
fn main() {
    let a = [0; 5];
    let ptr: *const i32 = &a[1];
    let _ = unsafe { ptr.offset(isize::MAX) };
}
