// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_div` intrinsic returns the expected results for floating point numbers.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_div;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, kani::Arbitrary)]
pub struct f32x2([f32; 2]);

impl f32x2 {
    fn new_with(f: impl Fn() -> f32) -> Self {
        f32x2([f(), f()])
    }

    fn non_simd_div(self, divisors: Self) -> Self {
        f32x2([self.0[0] / divisors.0[0], self.0[1] / divisors.0[1]])
    }
}

#[kani::proof]
fn test_simd_div() {
    let dividends = f32x2::new_with(|| {
        let multiplier = kani::any_where(|&n: &i8| n >= -5 && n <= 5);
        0.5 * f32::from(multiplier)
    });
    let divisors = f32x2::new_with(|| {
        let multiplier = kani::any_where(|&n: &i8| n != 0 && n >= -5 && n <= 5);
        0.5 * f32::from(multiplier)
    });
    let normal_results = dividends.non_simd_div(divisors);
    let simd_results = unsafe { simd_div(dividends, divisors) };
    assert_eq!(normal_results, simd_results);
}
