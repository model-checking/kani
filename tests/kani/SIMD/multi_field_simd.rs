// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify that Kani can properly handle SIMD declaration and field access using multi-field syntax.
//! Note: Multi-field SIMD is actually being deprecated, but until it's removed, we might
//! as well keep supporting it.
//! See <https://github.com/rust-lang/compiler-team/issues/621> for more details.

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
    assert!(x.into_array() != y.into_array());
}

#[kani::proof]
fn check_ge() {
    let x: i64x2 = kani::any();
    kani::assume(x.into_array()[0] > 0);
    kani::assume(x.into_array()[1] > 0);
    assert!(x > i64x2([0, 0]));
}
