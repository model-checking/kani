// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `copy_nonoverlapping` works as expected: Copies a number `n` of elements from
// pointer `src` to pointer `dst`. Their regions of memory do not overlap, otherwise the
// call to `copy_nonoverlapping` would fail (a separate test checks for this).

// These call the intrinsics themselves, which `core::ptr` wraps with extra debug assertions,
// so opt in to `core_intrinsics` instead of importing them through the accidentally stabilized
// `core::intrinsics` path (denied since rust-lang/rust#163160).
#![feature(core_intrinsics)]
#![allow(internal_features)]

#[kani::proof]
fn test_copy_nonoverlapping_simple() {
    let mut expected_val = 42;
    let src: *mut i32 = &mut expected_val as *mut i32;
    let mut old_val = 99;
    let dst: *mut i32 = &mut old_val;
    unsafe {
        core::intrinsics::copy_nonoverlapping(src, dst, 1);
        assert!(*dst == expected_val);
    }
}
