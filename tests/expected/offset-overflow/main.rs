// Copyright Kani Contributors
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
        let _d = offset(ptr, isize::MAX / 8);
    }
}
