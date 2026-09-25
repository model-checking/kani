// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! CBMC requires a numeric vector element type, so `Simd<*const T, N>` used to take the compiler
//! down with `assertion failed: typ.is_numeric()` before any harness ran:
//! https://github.com/model-checking/kani/issues/4867
//!
//! Pointer lanes are modeled as integers of the same width. Rust's SIMD types are array-based, so
//! a whole vector is usually reached through a byte-level reinterpretation, but `simd_extract`,
//! `simd_insert` and `simd_splat` do reach a single lane and have to convert between the integer
//! lane and the Rust pointer type. These harnesses check that an address survives the round trip
//! through each of those paths, including for a dereference.

#![feature(portable_simd, core_intrinsics)]

use std::intrinsics::simd::{simd_extract, simd_insert, simd_splat};
use std::simd::Simd;
use std::simd::ptr::SimdConstPtr;

#[kani::proof]
fn check_splat_and_len() {
    let v: Simd<*const i32, 2> = Simd::splat(core::ptr::null());
    assert!(v.as_array().len() == 2);
}

#[kani::proof]
fn check_null_lane() {
    let v: Simd<*const i32, 2> = Simd::splat(core::ptr::null());
    assert!(v.as_array()[0].is_null());
}

#[kani::proof]
fn check_addr_of_lanes() {
    let x = 7i32;
    let v: Simd<*const i32, 2> = Simd::splat(&x as *const i32);
    let addrs = v.addr();
    assert!(addrs[0] == addrs[1]);
}

#[kani::proof]
fn check_deref_after_round_trip() {
    let x = 7i32;
    let v: Simd<*const i32, 2> = Simd::splat(&x as *const i32);
    let p = v.as_array()[0];
    assert!(unsafe { *p } == 7);
}

/// The shape that blocks the standard library: `core` declares
/// `transmute_copy::<Simd<usize, 2>, Simd<*const i32, 2>>`.
#[kani::proof]
fn check_transmute_from_usize_vector() {
    let u: Simd<usize, 2> = Simd::splat(0);
    let v: Simd<*const i32, 2> = unsafe { core::mem::transmute_copy(&u) };
    assert!(v.as_array()[0].is_null());
}

#[kani::proof]
fn check_mut_ptr_lanes() {
    let mut x = 3i32;
    let v: Simd<*mut i32, 2> = Simd::splat(&mut x as *mut i32);
    let p = v.as_array()[1];
    unsafe { *p = 4 };
    assert!(x == 4);
}

/// `simd_extract` reads a single lane, so the integer lane has to be turned back into a pointer.
#[kani::proof]
fn check_extract_const_ptr_lane() {
    let x = 7i32;
    let y = 8i32;
    let v: Simd<*const i32, 2> = Simd::from_array([&x as *const i32, &y as *const i32]);
    let p0: *const i32 = unsafe { simd_extract(v, 0) };
    let p1: *const i32 = unsafe { simd_extract(v, 1) };
    assert!(unsafe { *p0 } == 7);
    assert!(unsafe { *p1 } == 8);
    assert!(p0 != p1);
}

/// A write through an extracted `*mut` lane has to reach the original object, so the lane must
/// carry provenance and not just an address.
#[kani::proof]
fn check_extract_mut_ptr_lane() {
    let mut x = 3i32;
    let v: Simd<*mut i32, 2> = Simd::splat(&mut x as *mut i32);
    let p: *mut i32 = unsafe { simd_extract(v, 1) };
    unsafe { *p = 4 };
    assert!(x == 4);
}

/// `simd_insert` writes a single lane; check the pointer lands in the requested lane only.
#[kani::proof]
fn check_insert_const_ptr_lane() {
    let x = 9i32;
    let v: Simd<*const i32, 2> = Simd::splat(core::ptr::null());
    let v: Simd<*const i32, 2> = unsafe { simd_insert(v, 1, &x as *const i32) };
    assert!(v.as_array()[0].is_null());
    assert!(!v.as_array()[1].is_null());
    let p: *const i32 = unsafe { simd_extract(v, 1) };
    assert!(unsafe { *p } == 9);
}

/// `simd_splat` is the other lane-wise conversion, in the same direction as `simd_insert`.
#[kani::proof]
fn check_splat_intrinsic_const_ptr() {
    let x = 5i32;
    let v: Simd<*const i32, 2> = unsafe { simd_splat(&x as *const i32) };
    let p: *const i32 = unsafe { simd_extract(v, 1) };
    assert!(unsafe { *p } == 5);
}

/// A lane extracted from a shuffled pointer vector still dereferences to the right object.
#[kani::proof]
fn check_extract_after_swizzle() {
    let a = 1i32;
    let b = 2i32;
    let v: Simd<*const i32, 2> = Simd::from_array([&a as *const i32, &b as *const i32]);
    let w: Simd<*const i32, 2> = std::simd::simd_swizzle!(v, [1, 0]);
    let p: *const i32 = unsafe { simd_extract(w, 0) };
    assert!(unsafe { *p } == 2);
}
