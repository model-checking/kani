// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that storing the result of a vector operation in a vector of
//! size different to the operands' sizes causes an error.
#![feature(repr_simd, platform_intrinsics)]

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
pub struct u32x4(u32, u32, u32, u32);

// From <https://github.com/rust-lang/rfcs/blob/master/text/1199-simd-infrastructure.md#comparisons>:
// > The type checker ensures that `T` and `U` have the same length, and that
// > `U` is appropriately "boolean"-y.
// This means that `U` is allowed to be `i64` or `u64`, but not `f64`.
extern "platform-intrinsic" {
    fn simd_eq<T, U>(x: T, y: T) -> U;
}

#[kani::proof]
fn main() {
    let x = u64x2(0, 0);
    let y = u64x2(0, 1);

    unsafe {
        let invalid_simd: u32x4 = simd_eq(x, y);
        assert!(invalid_simd == u32x4(u32::MAX, u32::MAX, 0, 0));
        // ^^^^ The code above fails to type-check in Rust with the error:
        // ```
        // error[E0511]: invalid monomorphization of `simd_eq` intrinsic: expected
        // return type with length 2 (same as input type `u64x2`), found `u32x4` with length 4
        // ```
    }
}
