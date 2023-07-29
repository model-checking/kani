// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify that Kani can properly handle SIMD declaration and field access using multi-field syntax.
//! Note: Multi-field SIMD is actually being deprecated, but until it's removed, we might
//! as well keep supporting it.
//! See <https://github.com/rust-lang/compiler-team/issues/621> for more details.

#![allow(non_camel_case_types)]
#![feature(repr_simd)]

#[repr(simd)]
#[derive(PartialEq, Eq, PartialOrd, kani::Arbitrary)]
pub struct i64x2(i64, i64);

#[kani::proof]
fn check_diff() {
    let x = i64x2(1, 2);
    let y = i64x2(3, 4);
    assert!(x != y);
}

#[kani::proof]
fn check_ge() {
    let x: i64x2 = kani::any();
    kani::assume(x.0 > 0);
    kani::assume(x.1 > 0);
    assert!(x > i64x2(0, 0));
}
