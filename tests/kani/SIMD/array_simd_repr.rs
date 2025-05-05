// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify that Kani can properly handle SIMD declaration and field access using array syntax.

#![allow(non_camel_case_types)]
#![feature(repr_simd)]

#[repr(simd)]
#[derive(Clone, PartialEq, Eq, PartialOrd, kani::Arbitrary)]
pub struct i64x2([i64; 2]);

#[kani::proof]
fn check_diff() {
    let x = i64x2([1, 2]);
    let y = i64x2([3, 4]);
    assert!(x != y);
}

#[kani::proof]
fn check_ge() {
    let x: i64x2 = kani::any();
    kani::assume(x.0[0] > 0);
    kani::assume(x.0[1] > 0);
    assert!(x > i64x2([0, 0]));
}

#[derive(Clone, Debug)]
#[repr(simd)]
struct CustomSimd<T, const LANES: usize>([T; LANES]);

#[kani::proof]
fn simd_vec() {
    let simd = CustomSimd([0u8; 10]);
    let idx: usize = kani::any_where(|x: &usize| *x < 10);
    assert_eq!(simd.0[idx], 0);
}
