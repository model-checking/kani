// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

//! Check that Kani flags a conversion of a NaN or INFINITY to an int via
//! `float_to_int_unchecked`

#[kani::proof]
fn check_nan() {
    let f: f32 = f32::NAN;
    let _i: u32 = unsafe { std::intrinsics::float_to_int_unchecked(f) };
}

#[kani::proof]
fn check_inf() {
    let f: f32 = f32::INFINITY;
    let _i: u32 = unsafe { std::intrinsics::float_to_int_unchecked(f) };
}

#[kani::proof]
fn check_neg_inf() {
    let f: f32 = f32::NEG_INFINITY;
    let _i: u32 = unsafe { std::intrinsics::float_to_int_unchecked(f) };
}
