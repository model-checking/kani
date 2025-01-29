// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that we emit an error when the third argument for
//! `simd_insert` (the value to be inserted) has a type different to the first
//! argument's (i.e., the vector) base type.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_insert;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2([i64; 2]);

#[kani::proof]
fn main() {
    let y = i64x2([0, 1]);
    let _ = unsafe { simd_insert(y, 0, 1) };
    // ^^^^ The code above fails to type-check in Rust with the error:
    // ```
    // error[E0511]: invalid monomorphization of `simd_insert` intrinsic: expected inserted type `i64` (element of input `i64x2`), found `i32`
    // ```
    //
    // The type assumed for the third argument (the value to be inserted) is
    // `i32`. It can be fixed by annotating the value with the type `i64`:
    //
    // ```rust
    // let _ = unsafe { simd_insert(y, 0, 1_i64) };
    // ```
}
