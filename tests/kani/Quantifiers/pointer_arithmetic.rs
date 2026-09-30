// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

//! Tests for pointer arithmetic inside quantifier predicates.
//! These exercise the intrinsic lowering in `inline_call_as_pure_expr`
//! which converts wrapping_byte_offset/wrapping_add to CBMC Plus.
//!
//! Quantifier ranges are half-open: `|i in (lo, hi)|` means `lo <= i < hi`.
//! All ranges below match the array lengths to stay in-bounds.

#[kani::proof]
fn check_wrapping_byte_offset_forall() {
    let arr: [u8; 8] = [0; 8];
    let ptr = arr.as_ptr();
    unsafe {
        assert!(kani::forall!(|i in (0, 8)| *ptr.wrapping_byte_offset(i as isize) == 0));
    }
}

#[kani::proof]
fn check_wrapping_byte_offset_exists() {
    let arr: [u8; 8] = [0, 0, 0, 42, 0, 0, 0, 0];
    let ptr = arr.as_ptr();
    unsafe {
        assert!(kani::exists!(|i in (0, 8)| *ptr.wrapping_byte_offset(i as isize) == 42));
    }
}

#[kani::proof]
fn check_wrapping_add_forall() {
    let arr: [u32; 4] = [10, 20, 30, 40];
    let ptr = arr.as_ptr();
    unsafe {
        assert!(kani::forall!(|i in (0, 4)| *ptr.wrapping_add(i) >= 10));
    }
}

/// Tests `ptr.wrapping_add(i)` with a different element type (u8 vs u32).
#[kani::proof]
fn check_wrapping_add_u8_forall() {
    let arr: [u8; 4] = [1, 2, 3, 4];
    let ptr = arr.as_ptr();
    unsafe {
        assert!(kani::forall!(|i in (0, 4)| *ptr.wrapping_add(i) > 0));
    }
}

#[kani::proof]
fn check_wrapping_add_exists() {
    let arr: [u32; 4] = [10, 20, 30, 40];
    let ptr = arr.as_ptr();
    unsafe {
        assert!(kani::exists!(|i in (0, 4)| *ptr.wrapping_add(i) == 30));
    }
}

/// Tests `wrapping_byte_offset` on a non-`u8` pointee.
/// Each `u32` is 4 bytes, so valid byte offsets are 0, 4, 8, 12.
/// This catches a bug where the offset was scaled by the pointee size
/// (would scale by 4) instead of by 1 (bytes).
#[kani::proof]
fn check_wrapping_byte_offset_u32() {
    let arr: [u32; 4] = [0x11111111, 0x22222222, 0x33333333, 0x44444444];
    let ptr = arr.as_ptr();
    unsafe {
        assert!(
            kani::forall!(|i in (0, 4)| *ptr.wrapping_byte_offset((i * 4) as isize) >= 0x11111111)
        );
    }
}

#[kani::proof]
fn check_wrapping_sub_forall() {
    let arr: [u32; 4] = [10, 20, 30, 40];
    let end_ptr = unsafe { arr.as_ptr().add(4) };
    unsafe {
        // end_ptr.wrapping_sub(i) for i in 1..=4 points to arr[4-i]
        assert!(kani::forall!(|i in (1, 5)| *end_ptr.wrapping_sub(i) >= 10));
    }
}

#[kani::proof]
fn check_wrapping_byte_sub_forall() {
    let arr: [u32; 4] = [10, 20, 30, 40];
    let end_ptr = unsafe { arr.as_ptr().add(4) };
    unsafe {
        // 4-byte stride: end_ptr.wrapping_byte_sub(i*4) for i in 1..=4
        assert!(kani::forall!(|i in (1, 5)| *end_ptr.wrapping_byte_sub(i * 4) >= 10));
    }
}
