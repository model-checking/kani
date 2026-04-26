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
