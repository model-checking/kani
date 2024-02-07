// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn verify_logf32() {
    let e = std::f32::consts::E;
    let e_log = e.ln();

    assert!((e_log - 1.0).abs() <= 0.1);
}

#[kani::proof]
fn verify_logf64() {
    let e = std::f64::consts::E;
    let e_log = e.ln();

    assert!((e_log - 1.0).abs() <= 0.1);
}
