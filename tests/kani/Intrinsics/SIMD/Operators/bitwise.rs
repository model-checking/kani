// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the following SIMD intrinsics are supported:
//!  * `simd_and`
//!  * `simd_or`
//!  * `simd_xor`
//! This is done by initializing vectors with the contents of 2-member tuples
//! with symbolic values. The result of using each of the intrinsics is compared
//! against the result of using the associated bitwise operator on the tuples.
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

extern "platform-intrinsic" {
    fn simd_and<T>(x: T, y: T) -> T;
    fn simd_or<T>(x: T, y: T) -> T;
    fn simd_xor<T>(x: T, y: T) -> T;
}

#[kani::proof]
fn test_simd_and() {
    let tup_x = (kani::any(), kani::any());
    let tup_y = (kani::any(), kani::any());
    let x = i64x2(tup_x.0, tup_x.1);
    let y = i64x2(tup_y.0, tup_y.1);
    let res_and = unsafe { simd_and(x, y) };
    assert_eq!(tup_x.0 & tup_y.0, res_and.0);
    assert_eq!(tup_x.1 & tup_y.1, res_and.1);
}

#[kani::proof]
fn test_simd_or() {
    let tup_x = (kani::any(), kani::any());
    let tup_y = (kani::any(), kani::any());
    let x = i64x2(tup_x.0, tup_x.1);
    let y = i64x2(tup_y.0, tup_y.1);
    let res_or = unsafe { simd_or(x, y) };
    assert_eq!(tup_x.0 | tup_y.0, res_or.0);
    assert_eq!(tup_x.1 | tup_y.1, res_or.1);
}

#[kani::proof]
fn test_simd_xor() {
    let tup_x = (kani::any(), kani::any());
    let tup_y = (kani::any(), kani::any());
    let x = i64x2(tup_x.0, tup_x.1);
    let y = i64x2(tup_y.0, tup_y.1);
    let res_xor = unsafe { simd_xor(x, y) };
    assert_eq!(tup_x.0 ^ tup_y.0, res_xor.0);
    assert_eq!(tup_x.1 ^ tup_y.1, res_xor.1);
}
