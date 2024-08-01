// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_shl` and `simd_shr` intrinsics are supported and they
//! return the expected results.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_shl, simd_shr};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2(i32, i32);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u32x2(u32, u32);

#[kani::proof]
fn test_simd_shl() {
    let value = kani::any();
    let values = i32x2(value, value);
    let shift = kani::any();
    kani::assume(shift >= 0);
    kani::assume(shift < 32);
    let shifts = i32x2(shift, shift);
    let normal_result = value << shift;
    let simd_result = unsafe { simd_shl(values, shifts) };
    assert_eq!(normal_result, simd_result.0);
}

#[kani::proof]
fn test_simd_shr_signed() {
    let value = kani::any();
    let values = i32x2(value, value);
    let shift = kani::any();
    kani::assume(shift >= 0);
    kani::assume(shift < 32);
    let shifts = i32x2(shift, shift);
    let normal_result = value >> shift;
    let simd_result = unsafe { simd_shr(values, shifts) };
    assert_eq!(normal_result, simd_result.0);
}

#[kani::proof]
fn test_simd_shr_unsigned() {
    let value = kani::any();
    let values = u32x2(value, value);
    let shift = kani::any();
    kani::assume(shift < 32);
    let shifts = u32x2(shift, shift);
    let normal_result = value >> shift;
    let simd_result = unsafe { simd_shr(values, shifts) };
    assert_eq!(normal_result, simd_result.0);
}
