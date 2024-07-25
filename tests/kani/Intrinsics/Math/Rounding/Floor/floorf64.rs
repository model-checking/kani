// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `floorf64` does return:
//  * The nearest integer below the argument for some concrete cases.
//  * A value that is closer to negative infinity in all cases.
//  * A value such that the difference between it and the argument is between
//    zero and one.
#![feature(core_intrinsics)]
use std::intrinsics::floorf64;

#[kani::proof]
fn test_one() {
    let one = 1.0;
    let result = unsafe { floorf64(one) };
    assert!(result == 1.0);
}

#[kani::proof]
fn test_one_frac() {
    let one_frac = 1.2;
    let result = unsafe { floorf64(one_frac) };
    assert!(result == 1.0);
}

#[kani::proof]
fn test_one_neg() {
    let one_neg = -1.8;
    let result = unsafe { floorf64(one_neg) };
    assert!(result == -2.0);
}

#[kani::proof]
fn test_conc() {
    let conc = -42.6;
    let result = unsafe { floorf64(conc) };
    assert!(result == -43.0);
}

#[kani::proof]
fn test_conc_sci() {
    let conc = 5.4e-2;
    let result = unsafe { floorf64(conc) };
    assert!(result == 0.0);
}

#[kani::proof]
#[kani::solver(minisat)]
fn test_towards_neg_inf() {
    let x: f64 = kani::any();
    kani::assume(!x.is_nan());
    let result = unsafe { floorf64(x) };
    assert!(result <= x);
}

#[kani::proof]
#[kani::solver(minisat)]
fn test_diff_one() {
    let x: f64 = kani::any();
    kani::assume(!x.is_nan());
    kani::assume(!x.is_infinite());
    let result = unsafe { floorf64(x) };
    let diff = (x - result).abs();
    // `diff` can be 1.0 if `x` is very small (e.g., -6.938894e-18)
    assert!(diff <= 1.0);
    assert!(diff >= 0.0);
}
