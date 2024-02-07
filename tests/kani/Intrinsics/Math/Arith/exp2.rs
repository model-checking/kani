// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test will trigger use of the `exp2f32` and `exp2f64` intrinsics, which in turn invoke
// functions modelled in CBMC's math library. These models use approximations as documented in
// CBMC's source code: https://github.com/diffblue/cbmc/blob/develop/src/ansi-c/library/math.c.

#[kani::proof]
fn verify_exp2_32() {
    let two = 2.0_f32;
    let two_two = two.exp2();

    assert!((two_two - 4.0).abs() <= 0.345);
}

#[kani::proof]
fn verify_exp2_64() {
    let two = 2.0_f64;
    let two_two = two.exp2();

    assert!((two_two - 4.0).abs() <= 0.345);
}
