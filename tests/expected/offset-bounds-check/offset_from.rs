// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani offset operations correctly detect out-of-bound access.

/// Verification should fail because safety violation is not a regular panic.
#[kani::proof]
#[kani::should_panic]
fn check_offset_from_oob_ptr() {
    let val = 10u128;
    let ptr: *const u128 = &val;
    let ptr_oob: *const u128 = ptr.wrapping_add(10);
    // SAFETY: This is not safe!
    let _offset = unsafe { ptr_oob.offset_from(ptr) };
}

#[kani::proof]
fn check_offset_from_diff_alloc() {
    let val1 = 10u128;
    let val2 = 0u128;
    let ptr1: *const u128 = &val1;
    let ptr2: *const u128 = &val2;
    // SAFETY: This is not safe!
    let offset = unsafe { ptr1.offset_from(ptr2) };
    assert!(offset != 0);
}

#[kani::proof]
#[kani::should_panic]
fn check_offset_from_unit_panic() {
    let val1 = kani::any();
    let val2 = kani::any();
    let ptr1: *const () = &val1 as *const _ as *const ();
    let ptr2: *const () = &val2 as *const _ as *const ();
    // SAFETY: This is safe but will panic...
    let _offset = unsafe { ptr1.offset_from(ptr2) };
}

#[kani::proof]
fn check_offset_from_same_dangling() {
    let val = 10u128;
    let ptr: *const u128 = &val;
    let ptr_oob_1: *const u128 = ptr.wrapping_add(10);
    let ptr_oob_2: *const u128 = ptr.wrapping_add(5).wrapping_add(5);
    // SAFETY: This is safe since the pointer is the same!
    let offset = unsafe { ptr_oob_1.offset_from(ptr_oob_2) };
    assert_eq!(offset, 0);
}
