// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_div` intrinsic returns the expected results for floating point numbers.
//! Checks that the `simd_rem` intrinsic exists.
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq)]
pub struct f32x2(f32, f32);

extern "platform-intrinsic" {
    fn simd_div<T>(x: T, y: T) -> T;
    fn simd_rem<T>(x: T, y: T) -> T;
}

#[kani::proof]
fn test_simd_div() {
    let dividend: f32 = kani::any::<i8>().into();
    let divisor: f32 = kani::any::<i8>().into();
    // Narrow down the divisor interval so the operation finishes in a short time
    kani::assume(divisor != 0.0 && divisor.abs() < 5.0);
    let normal_result = dividend / divisor;
    let dividends = f32x2(dividend, dividend);
    let divisors = f32x2(divisor, divisor);
    let simd_result = unsafe { simd_div(dividends, divisors) };
    assert_eq!(normal_result, simd_result.0);
}

#[kani::proof]
fn test_simd_rem() {
    let dividend: f32 = kani::any::<i8>().into();
    let divisor: f32 = kani::any::<i8>().into();
    // Narrow down the divisor interval so the operation finishes in a short time
    kani::assume(divisor != 0.0 && divisor.abs() < 5.0);
    let dividends = f32x2(dividend, dividend);
    let divisors = f32x2(divisor, divisor);
    let _simd_result = unsafe { simd_rem(dividends, divisors) };
}
