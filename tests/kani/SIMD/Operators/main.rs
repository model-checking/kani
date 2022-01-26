// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

extern "platform-intrinsic" {
    fn simd_add<T>(x: T, y: T) -> T;
    fn simd_sub<T>(x: T, y: T) -> T;
    fn simd_mul<T>(x: T, y: T) -> T;
    fn simd_div<T>(x: T, y: T) -> T;
    fn simd_rem<T>(x: T, y: T) -> T;
    fn simd_shl<T>(x: T, y: T) -> T;
    fn simd_shr<T>(x: T, y: T) -> T;
    fn simd_and<T>(x: T, y: T) -> T;
    fn simd_or<T>(x: T, y: T) -> T;
    fn simd_xor<T>(x: T, y: T) -> T;
}

macro_rules! assert_op {
    ($res_op: ident, $simd_op: ident, $x: expr, $y: expr, $($res: expr),+) => {
        let $res_op: i64x2 = $simd_op($x, $y);
        assert!($res_op == i64x2($($res),+))
    };
}

// Tests inspired by Rust's examples in
// https://github.com/rust-lang/rust/blob/0d97f7a96877a96015d70ece41ad08bb7af12377/src/test/ui/simd-intrinsic/simd-intrinsic-generic-arithmetic.rs
fn main() {
    let x = i64x2(0, 0);
    let y = i64x2(0, 1);
    let z = i64x2(1, 2);
    let v = i64x2(1, 3);

    unsafe {
        assert_op!(res_add, simd_add, x, y, 0, 1);
        assert_op!(res_sub, simd_sub, x, y, 0, -1);
        assert_op!(res_mul, simd_mul, y, z, 0, 2);
        assert_op!(res_div, simd_div, v, z, 1, 1);
        assert_op!(res_rem, simd_rem, v, z, 0, 1);
        assert_op!(res_shl, simd_shl, z, z, 2, 8);
        assert_op!(res_shr, simd_shr, z, y, 1, 1);
        assert_op!(res_and, simd_and, y, v, 0, 1);
        assert_op!(res_or, simd_or, x, y, 0, 1);
        assert_op!(res_xor, simd_xor, x, y, 0, 1);
    }
}
