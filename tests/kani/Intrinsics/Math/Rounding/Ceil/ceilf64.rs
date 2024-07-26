// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `ceilf64` does return:
//  * The nearest integer above the argument for some concrete cases.
//  * A value that is closer to infinity in all cases.
//  * A value such that the difference between it and the argument is between
//    zero and one.
#![feature(core_intrinsics)]
use std::intrinsics::ceilf64;

#[kani::proof]
fn test_one() {
    let one = 1.0;
    let ceil_res = unsafe { ceilf64(one) };
    assert!(ceil_res == 1.0);
}

#[kani::proof]
fn test_one_frac() {
    let one_frac = 1.2;
    let ceil_res = unsafe { ceilf64(one_frac) };
    assert!(ceil_res == 2.0);
}

#[kani::proof]
fn test_one_neg() {
    let one_neg = -1.8;
    let ceil_res = unsafe { ceilf64(one_neg) };
    assert!(ceil_res == -1.0);
}

#[kani::proof]
fn test_conc() {
    let conc = -42.6;
    let ceil_res = unsafe { ceilf64(conc) };
    assert!(ceil_res == -42.0);
}

#[kani::proof]
fn test_conc_sci() {
    let conc = 5.4e-2;
    let ceil_res = unsafe { ceilf64(conc) };
    assert!(ceil_res == 1.0);
}

#[kani::proof]
#[kani::solver(minisat)]
fn test_towards_inf() {
    let x: f64 = kani::any();
    kani::assume(!x.is_nan());
    let ceil_res = unsafe { ceilf64(x) };
    assert!(ceil_res >= x);
}

#[kani::proof]
#[kani::solver(minisat)]
fn test_diff_one() {
    let x: f64 = kani::any();
    kani::assume(!x.is_nan());
    kani::assume(!x.is_infinite());
    let ceil_res = unsafe { ceilf64(x) };
    let diff = (x - ceil_res).abs();
    // `diff` can be 1.0 if `x` is very small (e.g., 5.220244e-54)
    assert!(diff <= 1.0);
    assert!(diff >= 0.0);
}
