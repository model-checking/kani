// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ensure we can handle SIMD defined in the standard library
//! FIXME: <https://github.com/model-checking/kani/issues/2631>
#![allow(non_camel_case_types)]
#![feature(repr_simd, platform_intrinsics, portable_simd)]
use std::simd::f32x4;

extern "platform-intrinsic" {
    fn simd_add<T>(x: T, y: T) -> T;
    fn simd_eq<T, U>(x: T, y: T) -> U;
}

#[repr(simd)]
#[derive(Clone, PartialEq, kani::Arbitrary)]
pub struct f32x2(f32, f32);

impl f32x2 {
    fn as_array(&self) -> &[f32; 2] {
        unsafe { &*(self as *const f32x2 as *const [f32; 2]) }
    }
}

#[kani::proof]
fn check_sum() {
    let a = f32x2(0.0, 0.0);
    let b = kani::any::<f32x2>();
    let sum = unsafe { simd_add(a.clone(), b) };
    assert_eq!(sum.as_array(), a.as_array());
}

#[kani::proof]
fn check_sum_portable() {
    let a = f32x4::splat(0.0);
    let b = f32x4::from_array(kani::any());
    // Cannot compare them directly: https://github.com/model-checking/kani/issues/2632
    assert_eq!((a + b).as_array(), b.as_array());
}
