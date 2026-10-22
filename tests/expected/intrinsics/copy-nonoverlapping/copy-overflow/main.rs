// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Checks that `copy_nonoverlapping` triggers an overflow failure if the `count`
//! argument can overflow a `usize`

// These call the intrinsics themselves, which `core::ptr` wraps with extra debug assertions,
// so opt in to `core_intrinsics` instead of importing them through the accidentally stabilized
// `core::intrinsics` path (denied since rust-lang/rust#163160).
#![feature(core_intrinsics)]
#![allow(internal_features)]

#[kani::proof]
fn test_copy_nonoverlapping_unaligned() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();
    // Passing `max_count` is guaranteed to overflow
    // the count in bytes for `i32` pointers
    let max_count = usize::MAX / 4 + 1;

    unsafe {
        let dst = src.add(1) as *mut i32;
        core::intrinsics::copy_nonoverlapping(src, dst, max_count);
    }
}
