// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `roundf32` does return:
//  * The nearest integer to the argument for some concrete cases.
//  * A value that is closer to one of the limits (zero, infinity or negative
//    infinity, based on the fractional part of the argument) in all cases.
//  * A value such that the difference between it and the argument is between
//    zero and 0.5.
#![feature(core_intrinsics)]
use std::intrinsics::roundf32;

#[kani::proof]
fn test_one() {
    let one = 1.0;
    let result = unsafe { roundf32(one) };
    assert!(result == 1.0);
}

#[kani::proof]
fn test_one_frac() {
    let one_frac = 1.9;
    let result = unsafe { roundf32(one_frac) };
    assert!(result == 2.0);
}

#[kani::proof]
fn test_conc() {
    let conc = -42.6;
    let result = unsafe { roundf32(conc) };
    assert!(result == -43.0);
}

#[kani::proof]
fn test_conc_sci() {
    let conc = 5.4e-2;
    let result = unsafe { roundf32(conc) };
    assert!(result == 0.0);
}

#[kani::proof]
fn test_towards_closer() {
    let x: f32 = kani::any();
    kani::assume(!x.is_nan());
    kani::assume(!x.is_infinite());
    let result = unsafe { roundf32(x) };
    let frac = x.fract().abs();
    if x.is_sign_positive() {
        if frac >= 0.5 {
            assert!(result > x);
        } else {
            assert!(result <= x);
        }
    } else {
        if frac >= 0.5 {
            assert!(result < x);
        } else {
            assert!(result >= x);
        }
    }
}

#[kani::proof]
fn test_diff_half_one() {
    let x: f32 = kani::any();
    kani::assume(!x.is_nan());
    kani::assume(!x.is_infinite());
    let result = unsafe { roundf32(x) };
    let diff = (x - result).abs();
    assert!(diff <= 0.5);
    assert!(diff >= 0.0);
}
