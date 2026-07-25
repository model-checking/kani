// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `volatile_copy_memory` and `volatile_copy_nonoverlapping_memory`
// copy `count` elements from `src` to `dst`.
//
// `volatile_copy_memory` further allows the `src`/`dst` regions to overlap
// (memmove semantics), while `volatile_copy_nonoverlapping_memory` requires
// disjoint regions (memcpy semantics; the failing overlapping case is
// checked separately under
// `tests/expected/intrinsics/volatile_copy/overlapping`). Exercising the
// overlapping case here for `volatile_copy_memory` also distinguishes the
// two variants from one another: if they were accidentally coded up
// identically (e.g. both using `memcpy`), this proof would fail.
#![feature(core_intrinsics)]

#[kani::proof]
fn test_volatile_copy_memory_simple() {
    let mut expected_val = 42;
    let src: *mut i32 = &mut expected_val as *mut i32;
    let mut old_val = 99;
    let dst: *mut i32 = &mut old_val;
    unsafe {
        core::intrinsics::volatile_copy_memory(dst, src, 1);
        assert!(*dst == expected_val);
    }
}

#[kani::proof]
fn test_volatile_copy_memory_with_overlap() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // `dst` overlaps `src` in `arr[1]`. `volatile_copy_memory` must
        // still succeed here (unlike `volatile_copy_nonoverlapping_memory`).
        let dst = src.add(1) as *mut i32;
        core::intrinsics::volatile_copy_memory(dst, src, 2);
        // The first value does not change
        assert!(arr[0] == 0);
        // The next values are copied from `arr[0..=1]`
        assert!(arr[1] == 0);
        assert!(arr[2] == 1);
    }
}

#[kani::proof]
fn test_volatile_copy_nonoverlapping_memory_simple() {
    let mut expected_val = 42;
    let src: *mut i32 = &mut expected_val as *mut i32;
    let mut old_val = 99;
    let dst: *mut i32 = &mut old_val;
    unsafe {
        core::intrinsics::volatile_copy_nonoverlapping_memory(dst, src, 1);
        assert!(*dst == expected_val);
    }
}
