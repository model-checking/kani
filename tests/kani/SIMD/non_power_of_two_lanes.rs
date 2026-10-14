// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! `core::simd::Simd` is `#[repr(simd, packed)]`, so rustc lays out one whose lane count is not a
//! power of two as memory with no padding rather than as a vector. Kani's vector lowering only
//! expected the latter and crashed on every such `Simd`, including the NEON conversions in `core`
//! such as `From<int16x4x3_t> for Simd<i16, 12>`.
#![feature(portable_simd)]

use std::mem::{offset_of, size_of};
use std::simd::Simd;
use std::simd::cmp::SimdPartialEq;

#[kani::proof]
fn check_arithmetic_and_lanes() {
    let a: [i16; 12] = kani::any();
    let b: [i16; 12] = kani::any();
    kani::assume(a.iter().all(|x| *x > -1000 && *x < 1000));
    kani::assume(b.iter().all(|x| *x > -1000 && *x < 1000));
    let sum = Simd::from_array(a) + Simd::from_array(b);
    let i: usize = kani::any_where(|i| *i < 12);
    assert_eq!(sum[i], a[i] + b[i]);
    assert_eq!(Simd::from_array(a).to_array(), a);
}

#[kani::proof]
fn check_mask() {
    let a: [u8; 3] = kani::any();
    let b: [u8; 3] = kani::any();
    let eq = Simd::from_array(a).simd_eq(Simd::from_array(b));
    assert_eq!(eq.all(), a == b);
}

#[kani::proof]
fn check_transmute() {
    let x: [u8; 3] = kani::any();
    let v: Simd<u8, 3> = unsafe { std::mem::transmute(x) };
    assert_eq!(v, Simd::from_array(x));
    let back: [u8; 3] = unsafe { std::mem::transmute(v) };
    assert_eq!(back, x);
}

#[repr(C)]
struct Packed {
    v: Simd<i16, 12>,
    tail: u8,
}

#[kani::proof]
fn check_size_and_offsets() {
    assert_eq!(size_of::<Simd<i16, 12>>(), 24);
    assert_eq!(size_of::<Simd<u8, 3>>(), 3);
    assert_eq!(offset_of!(Packed, tail), 24);
    let p = Packed { v: Simd::splat(kani::any()), tail: kani::any() };
    let tail = p.tail;
    let copy = Packed { v: p.v, tail: p.tail };
    assert_eq!(copy.tail, tail);
    assert_eq!(copy.v, p.v);
}

#[kani::proof]
fn check_pointer_lanes() {
    let x = [1u8, 2, 3];
    let ptrs: Simd<*const u8, 3> = Simd::from_array([&x[0], &x[1], &x[2]]);
    let i: usize = kani::any_where(|i| *i < 3);
    assert_eq!(unsafe { *ptrs[i] }, x[i]);
}
