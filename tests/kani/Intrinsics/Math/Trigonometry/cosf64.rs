// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `cosf64` returns the expected results.
// Note: The CBMC model for this function is an over-approximation that returns:
//  * A nondet. value between -1 and 1
//  * 1.0 if the argument is 0.0
#![feature(core_intrinsics)]

fn fp_equals(value: f64, expected: f64) -> bool {
    let abs_diff = (value - expected).abs();
    abs_diff <= f64::EPSILON
}

#[kani::proof]
fn cosine_range() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite());
    let cosine = unsafe { std::intrinsics::cosf64(x) };
    assert!(cosine < 1.0 || fp_equals(cosine, 1.0));
    assert!(cosine > -1.0 || fp_equals(cosine, -1.0));
}

#[kani::proof]
fn cosine_const() {
    let x = 0.0;
    let cosine = unsafe { std::intrinsics::cosf64(x) };
    assert!(fp_equals(cosine, 1.0));
}
