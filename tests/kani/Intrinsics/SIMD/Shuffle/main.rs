// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_shuffle` and `simd_shuffleN` (where `N` is a length) are
//! supported and return the expected results.
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
    fn simd_shuffle<T, U, V>(x: T, y: T, idx: U) -> V;
}

#[kani::proof]
fn main() {
    {
        let y = i64x2(0, 1);
        let z = i64x2(1, 2);
        const I: [u32; 2] = [1, 2];
        let x: i64x2 = unsafe { simd_shuffle(y, z, I) };
        assert!(x.0 == 1);
        assert!(x.1 == 1);
    }
    {
        let y = i64x2(0, 1);
        let z = i64x2(1, 2);
        const I: [u32; 2] = [1, 2];
        let x: i64x2 = unsafe { simd_shuffle(y, z, I) };
        assert!(x.0 == 1);
        assert!(x.1 == 1);
    }
    {
        let a = i64x4(1, 2, 3, 4);
        let b = i64x4(5, 6, 7, 8);
        const I: [u32; 4] = [1, 3, 5, 7];
        let c: i64x4 = unsafe { simd_shuffle(a, b, I) };
        assert!(c == i64x4(2, 4, 6, 8));
    }
}
