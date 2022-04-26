// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `offset` fails if the offset computation would
// result in an arithmetic overflow
#![feature(core_intrinsics)]
use std::intrinsics::offset;

#[kani::proof]
fn test_offset_overflow() {
    let s: &str = "123";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        let _ = offset(ptr, isize::MAX);
    }
}
