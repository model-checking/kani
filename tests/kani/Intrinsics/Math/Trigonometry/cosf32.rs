// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `cosf32` returns the expected results.
// Note: The CBMC model for this function is an over-approximation that returns:
//  * A nondet. value between -1 and 1
//  * 1.0 if the argument is 0.0
#![feature(core_intrinsics)]

fn fp_equals(value: f32, expected: f32) -> bool {
    let abs_diff = (value - expected).abs();
    abs_diff <= f32::EPSILON
}

#[kani::proof]
fn cosine_range() {
    let x: f32 = kani::any();
    kani::assume(x.is_finite());
    let cosine = unsafe { std::intrinsics::cosf32(x) };
    assert!(cosine < 1.0 || fp_equals(cosine, 1.0));
    assert!(cosine > -1.0 || fp_equals(cosine, -1.0));
}

#[kani::proof]
fn cosine_const() {
    let x = 0.0;
    let cosine = unsafe { std::intrinsics::cosf32(x) };
    assert!(fp_equals(cosine, 1.0));
}
