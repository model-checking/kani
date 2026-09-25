// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! CBMC requires a numeric vector element type, so `Simd<*const T, N>` used to take the compiler
//! down with `assertion failed: typ.is_numeric()` before any harness ran:
//! https://github.com/model-checking/kani/issues/4867
//!
//! Pointer lanes are modeled as integers of the same width. Rust's SIMD types are array-based, so
//! the lanes are reached through a byte-level reinterpretation of the whole vector -- these
//! harnesses check that an address survives the round trip, including for a dereference.

#![feature(portable_simd)]

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
