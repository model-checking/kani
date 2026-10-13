// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `copy_nonoverlapping` fails if the `src`/`dst` regions overlap.

// These call the intrinsics themselves, which `core::ptr` wraps with extra debug assertions,
// so opt in to `core_intrinsics` instead of importing them through the accidentally stabilized
// `core::intrinsics` path (denied since rust-lang/rust#163160).
#![feature(core_intrinsics)]
#![allow(internal_features)]

#[kani::proof]
fn test_copy_nonoverlapping_with_overlap() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // The call to `copy_nonoverlapping` is expected to fail because
        // the `src` region and the `dst` region overlap in `arr[1]`
        let dst = src.add(1) as *mut i32;
        core::intrinsics::copy_nonoverlapping(src, dst, 2);
    }
}
