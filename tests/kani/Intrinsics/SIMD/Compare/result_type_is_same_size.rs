// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that storing the result of a vector operation in a vector of
//! size equal to the operands' sizes works.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_eq;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u64x2(u64, u64);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u32x2(u32, u32);

#[kani::proof]
fn main() {
    let x = u64x2(0, 0);
    let y = u64x2(0, 1);

    unsafe {
        let w: i64x2 = simd_eq(x, y);
        assert!(w == i64x2(-1, 0));

        let z: u32x2 = simd_eq(x, y);
        assert!(z == u32x2(u32::MAX, 0));
    }
}
