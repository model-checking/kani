// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `sinf32` returns the expected results.

//
// The CBMC model for `sinf32` is an overapproximation that returns:
//  * 0.0 if the argument is 0.0
//  * A symbolic value between -1.0 and 1.0 otherwise
#![feature(core_intrinsics)]

fn fp_equals(value: f32, expected: f32) -> bool {
    let abs_diff = (value - expected).abs();
    abs_diff <= f32::EPSILON
}

#[kani::proof]
fn sine_range() {
    let x: f32 = kani::any();
    kani::assume(x.is_finite());
    let sine = unsafe { std::intrinsics::sinf32(x) };
    assert!(sine < 1.0 || fp_equals(sine, 1.0));
    assert!(sine > -1.0 || fp_equals(sine, -1.0));
}

#[kani::proof]
fn sine_const() {
    let x = 0.0;
    let sine = unsafe { std::intrinsics::sinf32(x) };
    assert!(fp_equals(sine, 0.0));
}
