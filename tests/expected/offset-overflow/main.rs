// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `offset` fails if the offset computation would
// result in an arithmetic overflow
#![feature(core_intrinsics)]
use std::intrinsics::offset;

#[kani::proof]
fn test_offset_overflow() {
    let a: [i32; 3] = [1, 2, 3];
    let ptr: *const i32 = a.as_ptr();

    // a value that when multiplied by the size of i32 (i.e. 4 bytes)
    // would overflow `isize`
    let count: isize = isize::MAX / 4 + 1;
    unsafe {
        let _d = offset(ptr, count);
    }
}
