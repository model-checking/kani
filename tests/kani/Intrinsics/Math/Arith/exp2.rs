// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_exp2_32() {
    let two = 2.0_f32;
    let two_two = two.exp2();

    assert!((two_two - 4.0).abs() <= 1.0);
}

#[kani::proof]
fn verify_exp2_64() {
    let two = 2.0_f64;
    let two_two = two.exp2();

    assert!((two_two - 4.0).abs() <= 1.0);
}
