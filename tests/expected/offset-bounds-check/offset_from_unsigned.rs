// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can detect UB due to `offset_from_unsigned` with out-of-bounds pointer or wrong order.

#![feature(ptr_sub_ptr)]

#[kani::proof]
fn check_offset_from_unsigned_self_oob() {
    let val = 10u128;
    let ptr: *const u128 = &val;
    let ptr_oob: *const u128 = ptr.wrapping_add(10);
    // SAFETY: This is not safe!
    let _offset = unsafe { ptr_oob.offset_from_unsigned(ptr) };
}

#[kani::proof]
fn check_offset_from_unsigned_oob_ptr() {
    let val = 10u128;
    let ptr: *const u128 = &val;
    let ptr_oob: *const u128 = ptr.wrapping_sub(10);
    // SAFETY: This is not safe!
    let _offset = unsafe { ptr.offset_from_unsigned(ptr_oob) };
}

#[kani::proof]
fn check_offset_from_unsigned_diff_alloc() {
    let val1 = kani::any();
    let val2 = kani::any();
    let ptr1: *const u128 = &val1;
    let ptr2: *const u128 = &val2;
    // SAFETY: This is not safe!
    let _offset = unsafe { ptr1.offset_from_unsigned(ptr2) };
}

#[kani::proof]
fn check_offset_from_unsigned_negative_result() {
    let val: [u8; 10] = kani::any();
    let ptr_first: *const _ = &val[0];
    let ptr_last: *const _ = &val[9];
    // SAFETY: This is safe!
    let offset_ok = unsafe { ptr_last.offset_from_unsigned(ptr_first) };

    // SAFETY: This is not safe!
    let offset_not_ok = unsafe { ptr_first.offset_from_unsigned(ptr_last) };

    // Just use the result.
    assert!(offset_ok != offset_not_ok);
}

#[kani::proof]
#[kani::should_panic]
fn check_offset_from_unsigned_unit_panic() {
    let val1 = kani::any();
    let val2 = kani::any();
    let ptr1: *const () = &val1 as *const _ as *const ();
    let ptr2: *const () = &val2 as *const _ as *const ();
    // SAFETY: This is safe but will panic...
    let _offset = unsafe { ptr1.offset_from_unsigned(ptr2) };
}

#[kani::proof]
fn check_offset_from_unsigned_same_dangling() {
    let val = 10u128;
    let ptr: *const u128 = &val;
    let ptr_oob_1: *const u128 = ptr.wrapping_add(10);
    let ptr_oob_2: *const u128 = ptr.wrapping_add(5).wrapping_add(5);
    // SAFETY: This is safe since the pointer is the same!
    let offset = unsafe { ptr_oob_1.offset_from_unsigned(ptr_oob_2) };
    assert_eq!(offset, 0);
}
