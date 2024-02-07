// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_log2_32() {
    let two = 2.0f32;

    // log2(2) - 1 == 0
    let abs_difference = (two.log2() - 1.0).abs();

    assert!(abs_difference <= f32::EPSILON);
}

#[kani::proof]
fn verify_log2_64() {
    let four = 4.0_f64;

    // log2(4) - 2 == 0
    let abs_difference = (four.log2() - 2.0).abs();

    assert!(abs_difference < 1e-10);
}
