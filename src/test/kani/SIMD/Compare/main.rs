// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

// The predicate type U in the functions below must
// be a SIMD type, otherwise we get a compilation error
extern "platform-intrinsic" {
    fn simd_eq<T, U>(x: T, y: T) -> U;
    fn simd_ne<T, U>(x: T, y: T) -> U;
    fn simd_lt<T, U>(x: T, y: T) -> U;
    fn simd_le<T, U>(x: T, y: T) -> U;
    fn simd_gt<T, U>(x: T, y: T) -> U;
    fn simd_ge<T, U>(x: T, y: T) -> U;
}

macro_rules! assert_cmp {
    ($res_cmp: ident, $simd_cmp: ident, $x: expr, $y: expr, $($res: expr),+) => {
        let $res_cmp: i64x2 = $simd_cmp($x, $y);
        assert!($res_cmp == i64x2($($res),+))
    };
}

// https://gcc.gnu.org/onlinedocs/gcc/Vector-Extensions.html
// Vectors are compared element-wise producing:
//  * 0 when comparison is false
//  * -1 (all bits set) otherwise
fn main() {
    let x = i64x2(0, 0);
    let y = i64x2(0, 1);

    // CBMC does not support comparison operators
    // so the below assertions are expected to fail
    unsafe {
        assert_cmp!(res_eq, simd_eq, x, x, -1, -1);
        assert_cmp!(res_eq, simd_eq, x, y, -1, 0);
        assert_cmp!(res_ne, simd_ne, x, x, 0, 0);
        assert_cmp!(res_ne, simd_ne, x, y, 0, -1);
        assert_cmp!(res_lt, simd_lt, x, x, 0, 0);
        assert_cmp!(res_lt, simd_lt, x, y, 0, -1);
        assert_cmp!(res_le, simd_le, x, x, -1, -1);
        assert_cmp!(res_le, simd_le, x, y, -1, -1);
        assert_cmp!(res_gt, simd_gt, x, x, 0, 0);
        assert_cmp!(res_gt, simd_gt, x, y, 0, 0);
        assert_cmp!(res_ge, simd_ge, x, x, -1, -1);
        assert_cmp!(res_ge, simd_ge, x, y, -1, 0);
    }
}
