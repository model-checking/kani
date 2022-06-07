// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::truncf32;

// Checks that `truncf32` does return:
//  * The integral part of a number for a couple of concrete cases.
//  * A value that is closer to zero in all cases.

#[kani::proof]
fn test_one() {
    let one = 1.0;
    let trunc_res = unsafe { truncf32(one) };
    assert!(trunc_res == 1.0);
}

#[kani::proof]
fn test_one_frac() {
    let one_frac = 1.9;
    let trunc_res = unsafe { truncf32(one_frac) };
    assert!(trunc_res == 1.0);
}

#[kani::proof]
fn test_towards_zero() {
    let x: f32 = kani::any();
    kani::assume(!x.is_nan());
    let trunc_res = unsafe { truncf32(x) };
    if x.is_sign_positive() {
        assert!(trunc_res <= x);
    } else {
        assert!(trunc_res >= x);
    }
}
