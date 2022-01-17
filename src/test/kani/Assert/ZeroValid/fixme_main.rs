// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

// The function zeroed() calls assert_zero_valid to mark that it is only defined to assign an
// all-zero bit pattern to a type T if this is a valid value. So the following is undefined.

use std::mem;
use std::ptr;
fn main() {
    let x: &mut i32 = unsafe { mem::zeroed() }; //< undefined (should fail)
    let p: *mut i32 = x;
    assert!(p == ptr::null_mut()); //< verifies with RMC
}
