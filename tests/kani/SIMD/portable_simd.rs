// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ensure we have basic support of portable SIMD.
#![feature(portable_simd)]

use std::simd::u64x16;

#[kani::proof]
fn check_sum_any() {
    let a = u64x16::splat(0);
    let b = u64x16::from_array(kani::any());
    // Cannot compare them directly: https://github.com/model-checking/kani/issues/2632
    assert_eq!((a + b).as_array(), b.as_array());
}
