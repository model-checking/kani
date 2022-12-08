// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_rem` triggers a failure when the divisor is zero
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2(i32, i32);

extern "platform-intrinsic" {
    fn simd_rem<T>(x: T, y: T) -> T;
}

#[kani::proof]
fn test_simd_rem() {
    let dividend = kani::any();
    let dividends = i32x2(dividend, dividend);
    let divisor = 0;
    let divisors = i32x2(divisor, divisor);
    let _ = unsafe { simd_rem(dividends, divisors) };
}
