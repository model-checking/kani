// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_shuffle` and `simd_shuffleN` (where `N` is a length) are
//! supported and return the expected results.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_shuffle;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2([i64; 2]);

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x4([i64; 4]);

#[repr(simd)]
struct SimdShuffleIdx<const LEN: usize>([u32; LEN]);

#[kani::proof]
fn main() {
    {
        let y = i64x2([0, 1]);
        let z = i64x2([1, 2]);
        const I: SimdShuffleIdx<2> = SimdShuffleIdx([1, 2]);
        let x: i64x2 = unsafe { simd_shuffle(y, z, I) };
        assert!(x.0[0] == 1);
        assert!(x.0[1] == 1);
    }
    {
        let y = i64x2([0, 1]);
        let z = i64x2([1, 2]);
        const I: SimdShuffleIdx<2> = SimdShuffleIdx([1, 2]);
        let x: i64x2 = unsafe { simd_shuffle(y, z, I) };
        assert!(x.0[0] == 1);
        assert!(x.0[1] == 1);
    }
    {
        let a = i64x4([1, 2, 3, 4]);
        let b = i64x4([5, 6, 7, 8]);
        const I: SimdShuffleIdx<4> = SimdShuffleIdx([1, 3, 5, 7]);
        let c: i64x4 = unsafe { simd_shuffle(a, b, I) };
        assert!(c == i64x4([2, 4, 6, 8]));
    }
}

#[kani::proof]
fn check_shuffle() {
    {
        let y = i64x2([0, 1]);
        let z = i64x2([1, 2]);
        const I: SimdShuffleIdx<4> = SimdShuffleIdx([1, 2, 0, 3]);
        let _x: i64x4 = unsafe { simd_shuffle(y, z, I) };
    }
}
