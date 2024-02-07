// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_log10_32() {
    let ten = 10.0f32;

    // log10(10) - 1 == 0
    let abs_difference = (ten.log10() - 1.0).abs();

    assert!(abs_difference <= f32::EPSILON);
}

#[kani::proof]
fn verify_log10_64() {
    let hundred = 100.0_f64;

    // log10(100) - 2 == 0
    let abs_difference = (hundred.log10() - 2.0).abs();

    assert!(abs_difference < 1e-10);
}
