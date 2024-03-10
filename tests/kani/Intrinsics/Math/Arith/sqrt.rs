// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_sqrt32() {
    let positive = 4.0_f32;
    let negative = -4.0_f32;
    let negative_zero = -0.0_f32;

    let abs_difference = (positive.sqrt() - 2.0).abs();

    assert!(abs_difference <= f32::EPSILON);
    assert!(negative.sqrt().is_nan());
    assert!(negative_zero.sqrt() == negative_zero);
}

#[kani::proof]
fn verify_sqrt64() {
    let positive = 4.0_f64;
    let negative = -4.0_f64;
    let negative_zero = -0.0_f64;

    let abs_difference = (positive.sqrt() - 2.0).abs();

    assert!(abs_difference <= 1e-10);
    assert!(negative.sqrt().is_nan());
    assert!(negative_zero.sqrt() == negative_zero);
}
