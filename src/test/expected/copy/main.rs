// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::ptr;

fn main() {
    // TODO: make an overlapping set of locations, and check that it does the right thing for the overlapping region too.
    // https://github.com/model-checking/rmc/issues/12
    let expected_val = 42;
    let src: *const i32 = &expected_val as *const i32;
    let mut old_val = 99;
    let dst: *mut i32 = &mut old_val;
    unsafe {
        ptr::copy(src, dst, 1);
        assert!(*dst == expected_val);
    }
}
