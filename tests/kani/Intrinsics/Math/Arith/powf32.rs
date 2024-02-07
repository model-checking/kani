// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test will trigger use of the `powf32` intrinsic, which in turn invoke functions modelled in
// CBMC's math library. These models use approximations as documented in CBMC's source code:
// https://github.com/diffblue/cbmc/blob/develop/src/ansi-c/library/math.c.

#[kani::proof]
fn verify_pow() {
    let x: f32 = kani::any();
    kani::assume(x.is_normal());
    kani::assume(x > 1.0 && x < u16::MAX.into());
    let x2 = x.powf(2.0);
    assert!(x2 >= 0.0);
}
