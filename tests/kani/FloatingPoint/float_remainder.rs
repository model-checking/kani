// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Regression test for https://github.com/model-checking/kani/issues/2669
//! Floating-point remainder (%) was producing incorrect results because
//! CBMC's integer `mod` was used instead of `floatbv_mod`.

/// Symbolic f32 remainder: verify the fmod invariant |result| < |divisor|.
/// Inputs are i8-to-f32 casts, so both positive and negative values are
/// exercised without introducing NaN/infinity edge cases.
#[kani::proof]
fn check_f32_rem() {
    let dividend: f32 = kani::any::<i8>().into();
    let divisor: f32 = kani::any::<i8>().into();
    kani::assume(divisor != 0.0);
    let result = dividend % divisor;
    // fmod invariant: |result| < |divisor|
    assert!(result.abs() < divisor.abs());
}

/// Symbolic f64 remainder: same fmod invariant for f64.
#[kani::proof]
fn check_f64_rem() {
    let a: f64 = kani::any::<i8>().into();
    let b: f64 = kani::any::<i8>().into();
    kani::assume(b != 0.0);
    let result = a % b;
    assert!(result.abs() < b.abs());
}
