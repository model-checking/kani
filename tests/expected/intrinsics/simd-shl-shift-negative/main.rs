// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_shl` returns a failure if the shift distance is negative.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_shl;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2(i32, i32);

#[kani::proof]
fn test_simd_shl() {
    let value = kani::any();
    let values = i32x2(value, value);
    let shift = kani::any();
    kani::assume(shift < 32);
    let shifts = i32x2(shift, shift);
    let _result = unsafe { simd_shl(values, shifts) };
}
