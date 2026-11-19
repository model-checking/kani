// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `copy_nonoverlapping` fails when `dst` is not valid for writes.

// These call the intrinsics themselves, which `core::ptr` wraps with extra debug assertions,
// so opt in to `core_intrinsics` instead of importing them through the accidentally stabilized
// `core::intrinsics` path (denied since rust-lang/rust#163160).
#![feature(core_intrinsics)]
#![allow(internal_features)]

#[kani::proof]
fn test_copy_nonoverlapping_invalid() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // Get an invalid pointer with an out-of-bounds offset
        let dst_invalid = src.add(3) as *mut i32;
        core::intrinsics::copy_nonoverlapping(src, dst_invalid, 1);
    }
}
