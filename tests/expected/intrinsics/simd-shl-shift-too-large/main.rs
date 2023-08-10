// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_shl` returns a failure if the shift distance is too large.
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2(i32, i32);

extern "platform-intrinsic" {
    fn simd_shl<T>(x: T, y: T) -> T;
}

#[kani::proof]
fn test_simd_shl() {
    let value = kani::any();
    let values = i32x2(value, value);
    let shift = kani::any();
    kani::assume(shift >= 0);
    let shifts = i32x2(shift, shift);
    let _result = unsafe { simd_shl(values, shifts) };
}
