// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_log2_32() {
    let two = 2.0f32;

    // log2(2) - 1 == 0
    let abs_difference = (two.log2() - 1.0).abs();

    // should be <= f32::EPSILON, but CBMC's approximation of log2 makes results less precise
    assert!(abs_difference <= 0.09);
}

#[kani::proof]
fn verify_log2_64() {
    let four = 4.0_f64;

    // log2(4) - 2 == 0
    let abs_difference = (four.log2() - 2.0).abs();

    assert!(abs_difference < 0.09);
}
