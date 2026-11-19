// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `copy` fails when `src` is not aligned.

// These call the intrinsics themselves, which `core::ptr` wraps with extra debug assertions,
// so opt in to `core_intrinsics` instead of importing them through the accidentally stabilized
// `core::intrinsics` path (denied since rust-lang/rust#163160).
#![feature(core_intrinsics)]
#![allow(internal_features)]

#[kani::proof]
fn test_copy_unaligned() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // Get an unaligned pointer with a single-byte offset
        let src_i8: *const i8 = src as *const i8;
        let src_unaligned = unsafe { src_i8.add(1) as *const i32 };
        let dst = src.add(1) as *mut i32;
        core::intrinsics::copy(src_unaligned, dst, 1);
    }
}
