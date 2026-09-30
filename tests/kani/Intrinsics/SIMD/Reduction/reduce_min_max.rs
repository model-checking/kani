// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_reduce_max` and `simd_reduce_min` intrinsics return the largest and
//! smallest lane, comparing unsigned lanes as unsigned and signed lanes as signed. `core`'s
//! aarch64 `is_ascii` reaches `simd_reduce_max` through `vmaxvq_u8`.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_reduce_max, simd_reduce_min};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct u8x16([u8; 16]);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct i32x4([i32; 4]);

#[kani::proof]
#[kani::unwind(17)]
fn check_reduce_max_u8() {
    let lanes: [u8; 16] = kani::any();
    let max: u8 = unsafe { simd_reduce_max(u8x16(lanes)) };
    assert_eq!(max, *lanes.iter().max().unwrap());
}

#[kani::proof]
#[kani::unwind(17)]
fn check_reduce_min_u8() {
    let lanes: [u8; 16] = kani::any();
    let min: u8 = unsafe { simd_reduce_min(u8x16(lanes)) };
    assert_eq!(min, *lanes.iter().min().unwrap());
}

#[kani::proof]
#[kani::unwind(5)]
fn check_reduce_max_i32() {
    let lanes: [i32; 4] = kani::any();
    let max: i32 = unsafe { simd_reduce_max(i32x4(lanes)) };
    assert_eq!(max, *lanes.iter().max().unwrap());
}

#[kani::proof]
#[kani::unwind(5)]
fn check_reduce_min_i32() {
    let lanes: [i32; 4] = kani::any();
    let min: i32 = unsafe { simd_reduce_min(i32x4(lanes)) };
    assert_eq!(min, *lanes.iter().min().unwrap());
}
