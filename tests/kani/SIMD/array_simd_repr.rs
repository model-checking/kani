// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify that Kani can properly handle SIMD declared using array syntax.

#![allow(non_camel_case_types)]
#![feature(repr_simd)]

#[repr(simd)]
#[derive(PartialEq, Eq, kani::Arbitrary)]
pub struct i64x2([i64; 2]);

#[kani::proof]
fn check_diff() {
    let x = i64x2([1, 2]);
    let y = i64x2([3, 4]);
    assert!(x != y);
}

#[kani::proof]
fn check_nondet() {
    let x: i64x2 = kani::any();
    let y: i64x2 = kani::any();
    kani::cover!(x != y);
    kani::cover!(x == y);
}

#[derive(Clone, Debug)]
#[repr(simd)]
struct CustomSimd<T, const LANES: usize>([T; LANES]);

#[kani::proof]
fn simd_vec() {
    std::hint::black_box(CustomSimd([0u8; 10]));
}
