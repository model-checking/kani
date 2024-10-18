// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `arith_offset` succeeds even if the offset computation would
// result in an arithmetic overflow as it uses wrapping:
// https://doc.rust-lang.org/std/intrinsics/fn.arith_offset.html
#![feature(core_intrinsics)]
use std::intrinsics::arith_offset;

#[kani::proof]
fn test_arith_offset_overflow() {
    let s: &str = "123";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        let _ = arith_offset(ptr, isize::MAX);
    }
}
