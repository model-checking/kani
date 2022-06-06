// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `ceilf32` does return:
//  * The nearest integer above the argument for a few concrete cases.
//  * A value that is closer to infinity in all cases.
#![feature(core_intrinsics)]
use std::intrinsics::ceilf32;

#[kani::proof]
fn test_one() {
    let one = 1.0;
    let ceil_res = unsafe { ceilf32(one) };
    assert!(ceil_res == 1.0);
}

#[kani::proof]
fn test_one_frac() {
    let one_frac = 1.2;
    let ceil_res = unsafe { ceilf32(one_frac) };
    assert!(ceil_res == 2.0);
}

#[kani::proof]
fn test_one_neg() {
    let one_frac = -1.8;
    let ceil_res = unsafe { ceilf32(one_frac) };
    assert!(ceil_res == -1.0);
}

#[kani::proof]
fn test_towards_inf() {
    let x: f32 = kani::any();
    kani::assume(!x.is_nan());
    let ceil_res = unsafe { ceilf32(x) };
    assert!(ceil_res >= x);
}
