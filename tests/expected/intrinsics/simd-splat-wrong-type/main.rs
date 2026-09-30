// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that we emit an error when the argument of `simd_splat`
//! (the value to be broadcast) has a type different to the return vector's
//! element type. `simd_splat<T, U>(value: U) -> T` has independent generic
//! parameters, so such an instantiation type-checks in the frontend and must
//! be rejected during codegen, like rustc's own backends do.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_splat;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct i64x2([i64; 2]);

#[kani::proof]
fn main() {
    let _: i64x2 = unsafe { simd_splat(0_i32) };
    // ^^^^ The value to broadcast has type `i32`, but the element type of
    // `i64x2` is `i64`. It can be fixed by annotating the value with the
    // type `i64`:
    //
    // ```rust
    // let _: i64x2 = unsafe { simd_splat(0_i64) };
    // ```
}
