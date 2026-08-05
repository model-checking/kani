// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify that Kani can properly handle SIMD declaration and field access using array syntax.

#![allow(non_camel_case_types)]
#![feature(repr_simd)]

#[repr(simd)]
#[derive(Clone, Copy, kani::Arbitrary)]
pub struct i64x2([i64; 2]);

impl i64x2 {
    fn into_array(self) -> [i64; 2] {
        unsafe { std::mem::transmute(self) }
    }
}

impl std::cmp::PartialEq for i64x2 {
    fn eq(&self, other: &Self) -> bool {
        self.into_array() == other.into_array()
    }
}

impl std::cmp::PartialOrd for i64x2 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.into_array().partial_cmp(&other.into_array())
    }
}

#[kani::proof]
fn check_diff() {
    let x = i64x2([1, 2]);
    let y = i64x2([3, 4]);
    assert!(x != y);
}

#[kani::proof]
fn check_ge() {
    let x: i64x2 = kani::any();
    kani::assume(x.into_array()[0] > 0);
    kani::assume(x.into_array()[1] > 0);
    assert!(x > i64x2([0, 0]));
}

#[derive(Copy)]
#[repr(simd)]
struct CustomSimd<T, const LANES: usize>([T; LANES]);

impl<T: Copy, const LANES: usize> Clone for CustomSimd<T, LANES> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, const LANES: usize> CustomSimd<T, LANES> {
    fn as_array(&self) -> &[T; LANES] {
        let p: *const Self = self;
        unsafe { &*p.cast::<[T; LANES]>() }
    }

    fn into_array(self) -> [T; LANES]
    where
        T: Copy,
    {
        *self.as_array()
    }
}

#[kani::proof]
fn simd_vec() {
    let simd = CustomSimd([0u8; 10]);
    let idx: usize = kani::any_where(|x: &usize| *x < 10);
    assert_eq!(simd.into_array()[idx], 0);
}
