// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `f32::cos` returns the expected results.
// (nightly-2026-09-22 removed the `cosf32` intrinsic that used to back it.)
//
// The CBMC model for `cos` is an overapproximation that returns:
//  * 1.0 if the argument is 0.0
//  * A symbolic value between -1.0 and 1.0 otherwise

fn fp_equals(value: f32, expected: f32) -> bool {
    let abs_diff = (value - expected).abs();
    abs_diff <= f32::EPSILON
}

#[kani::proof]
fn cosine_range() {
    let x: f32 = kani::any();
    kani::assume(x.is_finite());
    let cosine = x.cos();
    assert!(cosine < 1.0 || fp_equals(cosine, 1.0));
    assert!(cosine > -1.0 || fp_equals(cosine, -1.0));
}

#[kani::proof]
fn cosine_const() {
    let x: f32 = 0.0;
    let cosine = x.cos();
    assert!(fp_equals(cosine, 1.0));
}
