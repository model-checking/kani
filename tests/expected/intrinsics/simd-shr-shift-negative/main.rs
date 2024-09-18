// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_shr` returns a failure if the shift distance is negative.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_shr;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2([i32; 2]);

#[kani::proof]
fn test_simd_shr() {
    let value = kani::any();
    let values = i32x2([value, value]);
    let shift = kani::any();
    kani::assume(shift < 32);
    let shifts = i32x2([shift, shift]);
    let _result = unsafe { simd_shr(values, shifts) };
}
