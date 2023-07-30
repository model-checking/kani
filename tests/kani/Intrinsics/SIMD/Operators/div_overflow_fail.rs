// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that the `simd_div` and `simd_rem` intrinsics check for overflows.
// kani-verify-fail

#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2(i32, i32);

extern "platform-intrinsic" {
    fn simd_div<T>(x: T, y: T) -> T;
    fn simd_rem<T>(x: T, y: T) -> T;
}

#[kani::proof]
fn test_simd_div_overflow() {
    let dividend = kani::any();
    kani::assume(dividend == i32::MIN);
    let dividends = i32x2(dividend, dividend);
    let divisor = kani::any();
    kani::assume(divisor == -1);
    let divisors = i32x2(divisor, divisor);
    let _quotient = unsafe { simd_div(dividends, divisors) };
}

#[kani::proof]
fn test_simd_rem_overflow() {
    let dividend = kani::any();
    kani::assume(dividend == i32::MIN);
    let dividends = i32x2(dividend, dividend);
    let divisor = kani::any();
    kani::assume(divisor == -1);
    let divisors = i32x2(divisor, divisor);
    let _remainder = unsafe { simd_rem(dividends, divisors) };
}
