// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that the pointer computed with `arith_offset` causes a failure if it's
// dereferenced outside the object bounds
#![feature(core_intrinsics)]
use std::intrinsics::arith_offset;

#[kani::proof]
fn test_arith_offset() {
    let s: &str = "123";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        let x = *arith_offset(ptr, 4) as char;
    }
}
