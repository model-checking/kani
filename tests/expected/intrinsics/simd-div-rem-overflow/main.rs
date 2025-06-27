// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_div` and `simd_rem` intrinsics check for integer overflows.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_div, simd_rem};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i32x2([i32; 2]);

unsafe fn do_simd_div(dividends: i32x2, divisors: i32x2) -> i32x2 {
    simd_div(dividends, divisors)
}

unsafe fn do_simd_rem(dividends: i32x2, divisors: i32x2) -> i32x2 {
    simd_rem(dividends, divisors)
}

#[kani::proof]
fn test_simd_div_overflow() {
    let dividend = i32::MIN;
    let dividends = i32x2([dividend, dividend]);
    let divisor = -1;
    let divisors = i32x2([divisor, divisor]);
    let quotients = unsafe { do_simd_div(dividends, divisors) };
    assert_eq!(quotients.0[0], quotients.0[1]);
}

#[kani::proof]
fn test_simd_rem_overflow() {
    let dividend = i32::MIN;
    let dividends = i32x2([dividend, dividend]);
    let divisor = -1;
    let divisors = i32x2([divisor, divisor]);
    let remainders = unsafe { do_simd_rem(dividends, divisors) };
    assert_eq!(remainders.0[0], remainders.0[1]);
}
