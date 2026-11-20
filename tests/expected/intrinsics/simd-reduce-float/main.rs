// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_reduce_max` and `simd_reduce_min` on floating-point lanes, which the intrinsics
//! do not accept, are reported as unsupported.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_reduce_max, simd_reduce_min};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct f32x4([f32; 4]);

#[kani::proof]
fn check_reduce_max_f32() {
    let _max: f32 = unsafe { simd_reduce_max(f32x4([1.0, 2.0, 3.0, 4.0])) };
}

#[kani::proof]
fn check_reduce_min_f32() {
    let _min: f32 = unsafe { simd_reduce_min(f32x4([1.0, 2.0, 3.0, 4.0])) };
}
