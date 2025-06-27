// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
#![feature(f16)]
#![feature(f128)]

// Check that the `float_to_int_unchecked` intrinsic works as expected

use std::intrinsics::float_to_int_unchecked;

macro_rules! check_float_to_int_unchecked {
    ($float_ty:ty, $int_ty:ty) => {
        let f: $float_ty = kani::any_where(|f: &$float_ty| {
            f.is_finite() && *f > <$int_ty>::MIN as $float_ty && *f < <$int_ty>::MAX as $float_ty
        });
        let u: $int_ty = unsafe { float_to_int_unchecked(f) };
        assert_eq!(u as $float_ty, f.trunc());
    };
}

macro_rules! check_float_to_int_unchecked_no_assert {
    ($float_ty:ty, $int_ty:ty) => {
        let f: $float_ty = kani::any_where(|f: &$float_ty| {
            f.is_finite() && *f > <$int_ty>::MIN as $float_ty && *f < <$int_ty>::MAX as $float_ty
        });
        let _u: $int_ty = unsafe { float_to_int_unchecked(f) };
    };
}

#[kani::proof]
fn check_f16_to_int_unchecked() {
    check_float_to_int_unchecked_no_assert!(f16, u8);
    check_float_to_int_unchecked_no_assert!(f16, u16);
    check_float_to_int_unchecked_no_assert!(f16, u32);
    check_float_to_int_unchecked_no_assert!(f16, u64);
    check_float_to_int_unchecked_no_assert!(f16, u128);
    check_float_to_int_unchecked_no_assert!(f16, usize);
    check_float_to_int_unchecked_no_assert!(f16, i8);
    check_float_to_int_unchecked_no_assert!(f16, i16);
    check_float_to_int_unchecked_no_assert!(f16, i32);
    check_float_to_int_unchecked_no_assert!(f16, i64);
    check_float_to_int_unchecked_no_assert!(f16, i128);
    check_float_to_int_unchecked_no_assert!(f16, isize);
}

#[kani::proof]
fn check_f32_to_int_unchecked() {
    check_float_to_int_unchecked!(f32, u8);
    check_float_to_int_unchecked!(f32, u16);
    check_float_to_int_unchecked!(f32, u32);
    check_float_to_int_unchecked!(f32, u64);
    check_float_to_int_unchecked!(f32, u128);
    check_float_to_int_unchecked!(f32, usize);
    check_float_to_int_unchecked!(f32, i8);
    check_float_to_int_unchecked!(f32, i16);
    check_float_to_int_unchecked!(f32, i32);
    check_float_to_int_unchecked!(f32, i64);
    check_float_to_int_unchecked!(f32, i128);
    check_float_to_int_unchecked!(f32, isize);
}

#[kani::proof]
fn check_f64_to_int_unchecked() {
    check_float_to_int_unchecked!(f64, u8);
    check_float_to_int_unchecked!(f64, u16);
    check_float_to_int_unchecked!(f64, u32);
    check_float_to_int_unchecked!(f64, u64);
    check_float_to_int_unchecked!(f64, u128);
    check_float_to_int_unchecked!(f64, usize);
    check_float_to_int_unchecked!(f64, i8);
    check_float_to_int_unchecked!(f64, i16);
    check_float_to_int_unchecked!(f64, i32);
    check_float_to_int_unchecked!(f64, i64);
    check_float_to_int_unchecked!(f64, i128);
    check_float_to_int_unchecked!(f64, isize);
}

#[kani::proof]
fn check_f128_to_int_unchecked() {
    check_float_to_int_unchecked_no_assert!(f128, u8);
    check_float_to_int_unchecked_no_assert!(f128, u16);
    check_float_to_int_unchecked_no_assert!(f128, u32);
    check_float_to_int_unchecked_no_assert!(f128, u64);
    check_float_to_int_unchecked_no_assert!(f128, u128);
    check_float_to_int_unchecked_no_assert!(f128, usize);
    check_float_to_int_unchecked_no_assert!(f128, i8);
    check_float_to_int_unchecked_no_assert!(f128, i16);
    check_float_to_int_unchecked_no_assert!(f128, i32);
    check_float_to_int_unchecked_no_assert!(f128, i64);
    check_float_to_int_unchecked_no_assert!(f128, i128);
    check_float_to_int_unchecked_no_assert!(f128, isize);
}
