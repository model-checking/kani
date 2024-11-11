// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

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
