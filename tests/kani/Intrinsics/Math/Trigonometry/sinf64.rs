// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `f64::sin` returns the expected results.
// (nightly-2026-09-22 removed the `sinf64` intrinsic that used to back it.)
//
// The CBMC model for `sin` is an overapproximation that returns:
//  * 0.0 if the argument is 0.0
//  * A symbolic value between -1.0 and 1.0 otherwise

fn fp_equals(value: f64, expected: f64) -> bool {
    let abs_diff = (value - expected).abs();
    abs_diff <= f64::EPSILON
}

#[kani::proof]
fn sine_range() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite());
    let sine = x.sin();
    assert!(sine < 1.0 || fp_equals(sine, 1.0));
    assert!(sine > -1.0 || fp_equals(sine, -1.0));
}

#[kani::proof]
fn sine_const() {
    let x: f64 = 0.0;
    let sine = x.sin();
    assert!(fp_equals(sine, 0.0));
}
