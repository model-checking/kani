// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `simd_shuffle` triggers an out-of-bounds failure when any of the
//! indexes supplied is greater than the combined size of the input vectors.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_shuffle;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2([i64; 2]);

#[repr(simd)]
struct SimdShuffleIdx<const LEN: usize>([u32; LEN]);

#[kani::proof]
fn main() {
    let y = i64x2([0, 1]);
    let z = i64x2([1, 2]);
    // Only [0, 3] are valid indexes, 4 is out of bounds
    const I: SimdShuffleIdx<2> = SimdShuffleIdx([1, 4]);
    let _: i64x2 = unsafe { simd_shuffle(y, z, I) };
}
