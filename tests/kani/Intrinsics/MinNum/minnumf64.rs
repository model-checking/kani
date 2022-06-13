// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `minnumf64` returns the minimum of two values, except in the
// following cases:
//  * If one of the arguments is NaN, the other arguments is returned.
//  * If both arguments are NaN, NaN is returned.
#![feature(core_intrinsics)]

#[kani::proof]
fn test_general() {
    let x: f64 = kani::any();
    let y: f64 = kani::any();
    kani::assume(!x.is_nan() && !y.is_nan());
    let res = std::intrinsics::minnumf64(x, y);
    if x < y {
        assert!(res == x);
    } else {
        assert!(res == y);
    }
}

#[kani::proof]
fn test_one_nan() {
    let x: f64 = kani::any();
    let y: f64 = kani::any();
    kani::assume((x.is_nan() && !y.is_nan()) || (!x.is_nan() && y.is_nan()));
    let res = std::intrinsics::minnumf64(x, y);
    if x.is_nan() {
        assert!(res == y);
    } else {
        assert!(res == x);
    }
}

#[kani::proof]
fn test_both_nan() {
    let x: f64 = kani::any();
    let y: f64 = kani::any();
    kani::assume(x.is_nan() && y.is_nan());
    let res = std::intrinsics::minnumf64(x, y);
    assert!(res.is_nan());
}
