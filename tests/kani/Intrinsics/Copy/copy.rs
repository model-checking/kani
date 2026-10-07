// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `copy` works as expected: Copies a number `n` of elements from
// pointer `src` to pointer `dst`, even if their regions overlap.

// These call the intrinsics themselves, which `core::ptr` wraps with extra debug assertions,
// so opt in to `core_intrinsics` instead of importing them through the accidentally stabilized
// `core::intrinsics` path (denied since rust-lang/rust#163160).
#![feature(core_intrinsics)]
#![allow(internal_features)]

#[kani::proof]
fn test_copy_simple() {
    let mut expected_val = 42;
    let src: *mut i32 = &mut expected_val as *mut i32;
    let mut old_val = 99;
    let dst: *mut i32 = &mut old_val;
    unsafe {
        core::intrinsics::copy(src, dst, 1);
        assert!(*dst == expected_val);
    }
}

#[kani::proof]
fn test_copy_with_overlap() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        let dst = src.add(1) as *mut i32;
        core::intrinsics::copy(src, dst, 2);
        // The first value does not change
        assert!(arr[0] == 0);
        // The next values are copied from `arr[0..=1]`
        assert!(arr[1] == 0);
        assert!(arr[2] == 1);
    }
}
