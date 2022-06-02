// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `minnumf32` returns the minimum of two values, except in the
// following cases:
//  * If one of the arguments is NaN, the other arguments is returned.
//  * If both arguments are NaN, NaN is returned.
#![feature(core_intrinsics)]

#[kani::proof]
fn test_general() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();
    kani::assume(!x.is_nan() && !y.is_nan());
    let res = std::intrinsics::minnumf32(x, y);
    if x < y {
        assert!(res == x);
    } else {
        assert!(res == y);
    }
}

#[kani::proof]
fn test_one_nan() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();
    kani::assume((x.is_nan() && !y.is_nan()) || (!x.is_nan() && y.is_nan()));
    let res = std::intrinsics::minnumf32(x, y);
    assert!(!res.is_nan());
}

#[kani::proof]
fn test_both_nan() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();
    kani::assume(x.is_nan() && y.is_nan());
    let res = std::intrinsics::minnumf32(x, y);
    assert!(res.is_nan());
}
