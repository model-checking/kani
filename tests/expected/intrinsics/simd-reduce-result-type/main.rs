// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_reduce_max` and `simd_reduce_min` with a result type other than the lane type,
//! which rustc rejects with `E0511`, are reported as unsupported.

#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_reduce_max, simd_reduce_min};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct u8x4([u8; 4]);

#[kani::proof]
fn check_reduce_max_wider_result() {
    let _max: u32 = unsafe { simd_reduce_max(u8x4([1, 2, 3, 4])) };
}

#[kani::proof]
fn check_reduce_min_signed_result() {
    let _min: i8 = unsafe { simd_reduce_min(u8x4([1, 2, 3, 4])) };
}
