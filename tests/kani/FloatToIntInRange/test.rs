// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfloat-lib
#![feature(f128)]

//! This test checks that `kani::float::float_to_int_in_range` works as expected

#[kani::proof]
fn check_float_to_int_in_range() {
    let f: f32 = 5.6;
    let fits_in_u16 = kani::float::float_to_int_in_range::<f32, u16>(f);
    assert!(fits_in_u16);
    let i: u16 = unsafe { f.to_int_unchecked() };
    assert_eq!(i, 5);

    let f: f32 = 145.7;
    let fits_in_i8 = kani::float::float_to_int_in_range::<f32, i8>(f);
    // doesn't fit in `i8` because the value after truncation (`145.0`) is larger than `i8::MAX`
    assert!(!fits_in_i8);

    let f: f64 = 1e6;
    let fits_in_u32 = kani::float::float_to_int_in_range::<f64, u32>(f);
    // fits in `u32` because the value after truncation (`1e6`) is smaller than `u32::MAX`
    assert!(fits_in_u32);
    let i: u32 = unsafe { f.to_int_unchecked() };
    assert_eq!(i, 1_000_000);
}

/// The `f128` -> `i128` lower bound used to be wrong (-2^128 instead of
/// -(2^127 + 2^15)), so `float_to_int_in_range` accepted values whose
/// truncation is below `i128::MIN`.
/// See https://github.com/model-checking/kani/issues/4662
#[kani::proof]
fn check_f128_to_i128_in_range_boundary() {
    // i128::MIN is exactly representable in f128 and must be accepted...
    let f: f128 = i128::MIN as f128;
    assert!(kani::float::float_to_int_in_range::<f128, i128>(f));
    let i: i128 = unsafe { f.to_int_unchecked() };
    assert_eq!(i, i128::MIN);

    // ...but the next f128 below it (-(2^127 + 2^15), whose truncation is
    // out of range) must be rejected.
    let g: f128 = -170141183460469231731687303715884138496.0;
    assert!(!kani::float::float_to_int_in_range::<f128, i128>(g));
}

/// Symbolic variant, mirroring the harness in verify-rust-std that caught
/// the wrong bound: for any value that `float_to_int_in_range` accepts,
/// `to_int_unchecked` must agree with the saturating `as` cast.
#[kani::proof]
fn check_f128_to_i128_in_range_symbolic() {
    let f: f128 = kani::any();
    kani::assume(kani::float::float_to_int_in_range::<f128, i128>(f));
    let i: i128 = unsafe { f.to_int_unchecked() };
    assert_eq!(i, f as i128);
}
