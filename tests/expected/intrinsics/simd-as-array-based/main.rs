// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --no-assertion-reach-checks
//
//! Test for the `simd_as` intrinsic with array-based SIMD types.
//! This test verifies that the intrinsic correctly handles type conversions
//! between SIMD vectors with array-based representations, fixing issue #2264.

#![feature(repr_simd, core_intrinsics)]

use std::intrinsics::simd::simd_as;

#[derive(Copy)]
#[repr(simd)]
struct Vu32([u32; 2]);

impl Clone for Vu32 {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy)]
#[repr(simd)]
struct Vi32([i32; 2]);

impl Clone for Vi32 {
    fn clone(&self) -> Self {
        *self
    }
}

#[kani::proof]
fn test_simd_as_same_size() {
    unsafe {
        let u = Vu32([u32::MIN, u32::MAX]);
        let _i: Vi32 = simd_as(u);
    }
}
