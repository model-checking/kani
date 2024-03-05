// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that intrinsics for SIMD vectors of signed integers are supported
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::*;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

macro_rules! assert_cmp {
    ($res_cmp: ident, $simd_cmp: ident, $x: expr, $y: expr, $($res: expr),+) => {
        let $res_cmp: i64x2 = $simd_cmp($x, $y);
        assert!($res_cmp == i64x2($($res),+))
    };
}

// https://gcc.gnu.org/onlinedocs/gcc/Vector-Extensions.html
// Vectors are compared element-wise producing:
//  * All bits set (e.g., -1 in signed integers) if the result is false
//  * No bits set (e.g., 0 in signed integers) if the result is true
#[kani::proof]
fn main() {
    let x = i64x2(0, 0);
    let y = i64x2(0, 1);

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
