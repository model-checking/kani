// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that storing the result of a vector operation in a vector of
//! size different to the operands' sizes causes an error.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_eq;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2([i64; 2]);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u64x2([u64; 2]);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct u32x4([u32; 4]);

#[kani::proof]
fn main() {
    let x = u64x2([0, 0]);
    let y = u64x2([0, 1]);

    unsafe {
        let invalid_simd: u32x4 = simd_eq(x, y);
        assert!(invalid_simd == u32x4([u32::MAX, u32::MAX, 0, 0]));
        // ^^^^ The code above fails to type-check in Rust with the error:
        // ```
        // error[E0511]: invalid monomorphization of `simd_eq` intrinsic: expected
        // return type with length 2 (same as input type `u64x2`), found `u32x4` with length 4
        // ```
    }
}
