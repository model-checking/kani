// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_extract` and `simd_insert` intrinsics are supported
//! and return the expected results.
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::{simd_extract, simd_insert};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i64x2(i64, i64);

#[kani::proof]
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
        let m = unsafe { simd_insert(y, 0, 1_i64) };
        let n = unsafe { simd_insert(y, 1, 5_i64) };
        assert!(m.0 == 1 && m.1 == 1);
        assert!(n.0 == 0 && n.1 == 5);
        // Original unchanged
        assert!(y.0 == 0 && y.1 == 1);
    }
}
