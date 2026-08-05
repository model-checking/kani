// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that storing the result of a vector operation in a vector of
//! size equal to the operands' sizes works.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_eq;

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
#[derive(Clone, Copy)]
pub struct u64x2([u64; 2]);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct u32x2([u32; 2]);

impl u32x2 {
    fn into_array(self) -> [u32; 2] {
        unsafe { std::mem::transmute(self) }
    }
}

#[kani::proof]
fn main() {
    let x = u64x2([0, 0]);
    let y = u64x2([0, 1]);

    unsafe {
        let w: i64x2 = simd_eq(x, y);
        assert!(w.into_array() == i64x2([-1, 0]).into_array());

        let z: u32x2 = simd_eq(x, y);
        assert!(z.into_array() == u32x2([u32::MAX, 0]).into_array());
    }
}
