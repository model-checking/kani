// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ensure we can safely swizzle between SIMD types of different sizes.
#![feature(portable_simd)]

use std::simd::{simd_swizzle, u32x4, u32x8};

#[kani::proof]
fn harness_from_u32x4_to_u32x4() {
    let a = u32x4::from_array([0, 1, 2, 3]);
    let b = u32x4::from_array([4, 5, 6, 7]);
    let r: u32x4 = simd_swizzle!(a, b, [0, 1, 6, 7]);
    assert_eq!(r.to_array(), [0, 1, 6, 7]);
}

#[kani::proof]
fn harness_from_u32x4_to_u32x8() {
    let a = u32x4::from_array([0, 1, 2, 3]);
    let b = u32x4::from_array([4, 5, 6, 7]);
    let r: u32x8 = simd_swizzle!(a, b, [0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(r.to_array(), [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[kani::proof]
fn harness_from_u32x8_to_u32x4() {
    let a = u32x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
    let b = u32x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
    let r: u32x4 = simd_swizzle!(a, b, [0, 1, 2, 3]);
    assert_eq!(r.to_array(), [0, 1, 2, 3]);
}
