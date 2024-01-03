// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that Kani triggers an error when the result type doesn't have the
//! length expected from a `simd_shuffle` call.
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x4(i64, i64, i64, i64);

extern "platform-intrinsic" {
    fn simd_shuffle<T, I, U>(x: T, y: T, idx: I) -> U;
}

#[kani::proof]
fn main() {
    let y = i64x2(0, 1);
    let z = i64x2(1, 2);
    const I: [u32; 4] = [1, 2, 1, 2];
    let x: i64x2 = unsafe { simd_shuffle(y, z, I) };
    // ^^^^ The code above fails to type-check in Rust with the error:
    // ```
    // error[E0511]: invalid monomorphization of `simd_shuffle4` intrinsic: expected return type of length 4, found `i64x2` with length 2
    // ```
}
