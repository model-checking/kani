// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test will trigger use of the `logf32` and `logf64` intrinsics, which in turn invoke
// functions modelled in CBMC's math library. These models use approximations as documented in
// CBMC's source code: https://github.com/diffblue/cbmc/blob/develop/src/ansi-c/library/math.c.

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
