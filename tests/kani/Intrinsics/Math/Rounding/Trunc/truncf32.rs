// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `truncf32` does return:
//  * The integral part of the argument for some concrete cases.
//  * A value that is closer to zero in all cases.
//  * A value such that the difference between it and the argument is between
//    zero and one.
#![feature(core_intrinsics)]
use std::intrinsics::truncf32;

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
fn test_conc() {
    let conc = -42.6;
    let trunc_res = unsafe { truncf32(conc) };
    assert!(trunc_res == -42.0);
}

#[kani::proof]
fn test_conc_sci() {
    let conc = 5.4e-2;
    let trunc_res = unsafe { truncf32(conc) };
    assert!(trunc_res == 0.0);
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

#[kani::proof]
fn test_diff_one() {
    let x: f32 = kani::any();
    kani::assume(!x.is_nan());
    kani::assume(!x.is_infinite());
    let trunc_res = unsafe { truncf32(x) };
    let diff = (x - trunc_res).abs();
    assert!(diff < 1.0);
    assert!(diff >= 0.0);
}
