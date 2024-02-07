// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test will trigger use of the `expf32` and `expf64` intrinsics, which in turn invoke
// functions modelled in CBMC's math library. These models use approximations as documented in
// CBMC's source code: https://github.com/diffblue/cbmc/blob/develop/src/ansi-c/library/math.c.

#[kani::proof]
fn verify_exp32() {
    let two = 2.0_f32;
    let two_sq = std::f32::consts::E * std::f32::consts::E;
    let two_exp = two.exp();

    assert!((two_sq - two_exp).abs() <= 1.0);
}

#[kani::proof]
fn verify_exp64() {
    let two = 2.0_f64;
    let two_sq = std::f64::consts::E * std::f64::consts::E;
    let two_exp = two.exp();

    assert!((two_sq - two_exp).abs() <= 1.0);
}
