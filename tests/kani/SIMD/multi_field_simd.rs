// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Verify that Kani can properly handle SIMD declared using multi-field syntax.
//! Note: Multi-field SIMD is actually being deprecated, but until it's removed, we might
//! as well keep supporting it.
//! See <https://github.com/rust-lang/compiler-team/issues/621> for more details.

#![allow(non_camel_case_types)]
#![feature(repr_simd)]

#[repr(simd)]
#[derive(PartialEq, Eq, kani::Arbitrary)]
pub struct i64x2(i64, i64);

#[kani::proof]
fn check_diff() {
    let x = i64x2(1, 2);
    let y = i64x2(3, 4);
    assert!(x != y);
}

#[kani::proof]
fn check_nondet() {
    let x: i64x2 = kani::any();
    let y: i64x2 = kani::any();
    kani::cover!(x != y);
    kani::cover!(x == y);
}
