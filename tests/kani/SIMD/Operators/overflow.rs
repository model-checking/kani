// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-verify-fail
//
// This test ensures that overflow in SIMD operations are detected by Kani.
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct i8x2(i8, i8);

extern "platform-intrinsic" {
    fn simd_add<T>(x: T, y: T) -> T;
    fn simd_sub<T>(x: T, y: T) -> T;
    fn simd_mul<T>(x: T, y: T) -> T;
}

macro_rules! assert_op {
    ($simd_op: ident, $wrap_op: ident, $x: expr, $y: expr) => {
        let result = $simd_op($x, $y);
        assert!(result.0 == $x.0.$wrap_op($y.0));
        assert!(result.1 == $x.1.$wrap_op($y.1));
    };
}

// Tests inspired by Rust's examples in
// https://github.com/rust-lang/rust/blob/0d97f7a96877a96015d70ece41ad08bb7af12377/src/test/ui/simd-intrinsic/simd-intrinsic-generic-arithmetic.rs
#[kani::proof]
fn main() {
    let v1 = i8x2(2, 2);
    let max_min = i8x2(i8::MIN, i8::MAX);

    unsafe {
        assert_op!(simd_add, wrapping_add, v1, max_min);
        assert_op!(simd_sub, wrapping_sub, v1, max_min);
        assert_op!(simd_mul, wrapping_mul, v1, max_min);
    }
}
