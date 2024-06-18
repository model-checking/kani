// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_div` and `simd_rem` intrinsics are supported and they
//! return the expected results.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_div, simd_rem};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2(i32, i32);

#[kani::proof]
fn test_simd_div() {
    let dividend = kani::any();
    let dividends = i32x2(dividend, dividend);
    let divisor = kani::any();
    // Narrow down the divisor interval so the operation doesn't overflow and
    // the test finishes in a short time
    kani::assume(divisor > 0 && divisor < 5);
    let divisors = i32x2(divisor, divisor);
    let normal_result = dividend / divisor;
    let simd_result = unsafe { simd_div(dividends, divisors) };
    assert_eq!(normal_result, simd_result.0);
}

#[kani::proof]
fn test_simd_rem() {
    let dividend = kani::any();
    let dividends = i32x2(dividend, dividend);
    let divisor = kani::any();
    // Narrow down the divisor interval so the operation doesn't overflow and
    // the test finishes in a short time
    kani::assume(divisor > 0 && divisor < 5);
    let divisors = i32x2(divisor, divisor);
    let normal_result = dividend % divisor;
    let simd_result = unsafe { simd_rem(dividends, divisors) };
    assert_eq!(normal_result, simd_result.0);
}
