// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani correctly computes the value of a wrapping_offset in cases where
//! the add operation wraps.
//!
//! Note that CBMC offset logic will wrap around the object bits, not the entire address space, when
//! computing the offset between pointers. Doing that is UB in Rust, so we should be OK
//! as long as Kani can detect UB in that case.

use std::convert::TryInto;

#[kani::proof]
fn original_harness() {
    let v: &[u128] = &[0; 10];
    let v_0: *const u128 = &v[0];
    let high_offset = usize::MAX / (std::mem::size_of::<u128>() * 4);
    unsafe {
        let v_wrap: *const u128 = v_0.wrapping_add(high_offset);
        // This should trigger UB!!
        let wrapped_offset = unsafe { v_wrap.offset_from(v_0) };
        // Without UB detection, the offsets are the same, but CBMC pointer arithmetic
        // would "wrap around" making this incorrect
        // https://github.com/model-checking/kani/issues/1150
        assert!(high_offset == wrapped_offset.try_into().unwrap());
    }
}

/// This harness is similar to the `original_harness`, but we replace the `offset_from` with
/// a subtraction on the pointer addresses.
#[kani::proof]
fn harness_without_ub() {
    let v: &[u128] = &[0; 10];
    let v_0: *const u128 = &v[0];
    let high_offset = usize::MAX / (size_of::<u128>() * 4);
    unsafe {
        let v_wrap: *const u128 = v_0.wrapping_add(high_offset);
        // The only way to compute offset of pointers out of bounds is to convert them to integers.
        let wrapped_offset = (v_wrap.addr() - v_0.addr()) / size_of::<u128>();
        // Now this should work
        assert_eq!(high_offset, wrapped_offset);
    }
}

#[kani::proof]
fn check_wrap_ptr_max() {
    let v: &[u128] = &[0; 10];
    let orig_ptr: *const u128 = &v[0];
    let new_ptr: *const u128 = orig_ptr.wrapping_byte_add(usize::MAX).wrapping_byte_add(1);
    assert_eq!(orig_ptr as usize, new_ptr as usize);
}

#[kani::proof]
fn check_wrap_ptr_10_bits() {
    let v: &[u128] = &[0; 10];
    let orig_ptr: *const u128 = &v[0];
    let new_ptr: *const u128 = orig_ptr.wrapping_byte_add(1 << 63);
    assert_ne!(orig_ptr as usize, new_ptr as usize);
}
