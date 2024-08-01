// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test ensures we detect overflows in SIMD arithmetic operations
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_add, simd_mul, simd_sub};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct i8x2(i8, i8);

#[kani::proof]
fn main() {
    let a = kani::any();
    let b = kani::any();
    let simd_a = i8x2(a, a);
    let simd_b = i8x2(b, b);

    unsafe {
        let _ = simd_add(simd_a, simd_b);
        let _ = simd_sub(simd_a, simd_b);
        let _ = simd_mul(simd_a, simd_b);
    }
}
