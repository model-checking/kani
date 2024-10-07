// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that Kani triggers an error when the result type doesn't have the
//! subtype expected from a `simd_shuffle` call.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_shuffle;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq)]
pub struct i64x2([i64; 2]);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq)]
pub struct f64x2([f64; 2]);

#[kani::proof]
fn main() {
    let y = i64x2([0, 1]);
    let z = i64x2([1, 2]);
    const I: [u32; 2] = [1, 2];
    let x: f64x2 = unsafe { simd_shuffle(y, z, I) };
    // ^^^^ The code above fails to type-check in Rust with the error:
    // ```
    // error[E0511]: invalid monomorphization of `simd_shuffle2` intrinsic: expected return element type `i64` (element of input `i64x2`), found `f64x2` with element type `f64`
    // ```
}
