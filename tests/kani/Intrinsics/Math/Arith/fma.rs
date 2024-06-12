// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_fma_32() {
    let m = 10.0_f32;
    let x = 4.0_f32;
    let b = 60.0_f32;

    // 100.0
    let abs_difference = (m.mul_add(x, b) - ((m * x) + b)).abs();

    assert!(abs_difference <= f32::EPSILON);
}

#[kani::proof]
fn verify_fma_64() {
    let m = 10.0_f64;
    let x = 4.0_f64;
    let b = 60.0_f64;

    // 100.0
    let abs_difference = (m.mul_add(x, b) - ((m * x) + b)).abs();

    assert!(abs_difference < 1e-10);
}
