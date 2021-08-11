// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
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
    fn simd_extract<T, U>(x: T, idx: u32) -> U;
    fn simd_insert<T, U>(x: T, idx: u32, b: U) -> T;
    fn simd_shuffle2<T, U>(x: T, y: T, idx: [u32; 2]) -> U;
    fn simd_shuffle4<T, U>(x: T, y: T, idx: [u32; 4]) -> U;
}

fn main() {
    let y = i64x2(0, 1);
    let z = i64x2(1, 2);

    // Indexing into the vectors
    assert!(z.0 == 1);
    assert!(z.1 == 2);

    {
        // Intrinsic indexing
        let y_0: i64 = unsafe { simd_extract(y, 0) };
        let y_1: i64 = unsafe { simd_extract(y, 1) };
        assert!(y_0 == 0);
        assert!(y_1 == 1);
    }
    {
        // Intrinsic updating
        let m = unsafe { simd_insert(y, 0, 1) };
        let n = unsafe { simd_insert(y, 1, 5) };
        assert!(m.0 == 1 && m.1 == 1);
        assert!(n.0 == 0 && n.1 == 5);
        // Original unchanged
        assert!(y.0 == 0 && y.1 == 1);
    }
    {
        const I: [u32; 2] = [1, 2];
        let x: i64x2 = unsafe { simd_shuffle2(y, z, I) };
        assert!(x.0 == 1);
        assert!(x.1 == 1);
    }
    {
        let a = i64x4(1, 2, 3, 4);
        let b = i64x4(5, 6, 7, 8);
        const I: [u32; 4] = [1, 3, 5, 7];
        let c: i64x4 = unsafe { simd_shuffle4(a, b, I) };
        assert!(c == i64x4(2, 4, 6, 8));
    }
}
