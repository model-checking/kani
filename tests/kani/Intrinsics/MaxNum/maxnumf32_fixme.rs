// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `maxnumf32` returns the minimum of two values, except in the
// following cases:
//  * If one of the arguments is NaN, the other arguments is returned.
//  * If both arguments are NaN, NaN is returned.
#![feature(core_intrinsics)]

// Note: All test cases are failing with the following error:
// ```
// Check X: fmaxf.assertion.1
//          - Status: FAILURE
//          - Description: "Function with missing definition is unreachable"
//          - Location: <builtin-library-fmaxf> in function fmaxf
// ```
// This is because the `fmaxf` definition is not being found, either because
// Kani does not produce the right expression (which is strange, because it's
// doing the same for similar expressions and they work) or CBMC is not picking
// it for some reason.
// Tracked in https://github.com/model-checking/kani/issues/1025
#[kani::proof]
fn test_general() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();
    kani::assume(!x.is_nan() && !y.is_nan());
    let res = std::intrinsics::maxnumf32(x, y);
    if x > y {
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
    let res = std::intrinsics::maxnumf32(x, y);
    assert!(!res.is_nan());
}

#[kani::proof]
fn test_both_nan() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();
    kani::assume(x.is_nan() && y.is_nan());
    let res = std::intrinsics::maxnumf32(x, y);
    assert!(res.is_nan());
}
