// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that we emit an error when the return type for
//! `simd_extract` has a type different to the first argument's (i.e., the
//! vector) base type.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_extract;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2([i64; 2]);

#[kani::proof]
fn main() {
    let y = i64x2([0, 1]);
    let res: i32 = unsafe { simd_extract(y, 1) };
    // ^^^^ The code above fails to type-check in Rust with the error:
    // ```
    // error[E0511]: invalid monomorphization of `simd_extract` intrinsic: expected return type `i64` (element of input `i64x2`), found `i32`
    // ```
    //
    // The return type `i32` comes from the type annotation in `res`. It can be
    // fixed by annotating `res` with type `i64`:
    //
    // ```rust
    // let res: i64 = unsafe { simd_extract(y, 1) };
    // ```
}
