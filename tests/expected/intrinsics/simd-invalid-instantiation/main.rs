// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that an invalid instantiation of `simd_splat`, `simd_extract`, `simd_insert` or
//! `simd_shuffle` fails only the harness that reaches it, while the valid instantiations in the
//! same crate still verify. rustc's codegen backends reject the invalid ones with E0511; Kani used
//! to report a compile error, which aborted the whole crate (see #4952).
#![feature(repr_simd, core_intrinsics)]
#![allow(internal_features)]
use std::intrinsics::simd::{simd_extract, simd_insert, simd_shuffle, simd_splat};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct i64x2([i64; 2]);

impl i64x2 {
    fn into_array(self) -> [i64; 2] {
        unsafe { std::mem::transmute(self) }
    }
}

#[repr(simd)]
#[allow(non_camel_case_types)]
pub struct i32x2([i32; 2]);

#[repr(simd)]
#[allow(non_camel_case_types)]
pub struct f64x2([f64; 2]);

#[repr(simd)]
struct SimdShuffleIdx<const LEN: usize>([u32; LEN]);

#[kani::proof]
fn check_splat() {
    let v: i64x2 = unsafe { simd_splat(7_i64) };
    assert!(v.into_array() == [7, 7]);
}

#[kani::proof]
fn check_splat_wrong_argument_type() {
    let _: i64x2 = unsafe { simd_splat(7_i32) };
}

#[kani::proof]
fn check_extract() {
    let v = i64x2([3, 4]);
    let x: i64 = unsafe { simd_extract(v, 1) };
    assert!(x == 4);
}

#[kani::proof]
fn check_extract_wrong_result_type() {
    let v = i64x2([3, 4]);
    let _: i32 = unsafe { simd_extract(v, 1) };
}

#[kani::proof]
fn check_insert() {
    let v = i64x2([3, 4]);
    let w = unsafe { simd_insert(v, 0, 5_i64) };
    assert!(w.into_array() == [5, 4]);
}

#[kani::proof]
fn check_insert_wrong_value_type() {
    let v = i64x2([3, 4]);
    let _ = unsafe { simd_insert(v, 0, 5_i32) };
}

#[kani::proof]
fn check_shuffle() {
    let a = i64x2([10, 11]);
    let b = i64x2([20, 21]);
    const I: SimdShuffleIdx<2> = SimdShuffleIdx([1, 2]);
    let c: i64x2 = unsafe { simd_shuffle(a, b, I) };
    assert!(c.into_array() == [11, 20]);
}

#[kani::proof]
fn check_shuffle_wrong_index_type() {
    let a = i64x2([10, 11]);
    let b = i64x2([20, 21]);
    const I: i32x2 = i32x2([1, 2]);
    let _: i64x2 = unsafe { simd_shuffle(a, b, I) };
}

#[kani::proof]
fn check_shuffle_wrong_result_lane_count() {
    let a = i64x2([10, 11]);
    let b = i64x2([20, 21]);
    const I: SimdShuffleIdx<4> = SimdShuffleIdx([1, 2, 1, 2]);
    let _: i64x2 = unsafe { simd_shuffle(a, b, I) };
}

#[kani::proof]
fn check_shuffle_wrong_result_lane_type() {
    let a = i64x2([10, 11]);
    let b = i64x2([20, 21]);
    const I: SimdShuffleIdx<2> = SimdShuffleIdx([1, 2]);
    let _: f64x2 = unsafe { simd_shuffle(a, b, I) };
}
