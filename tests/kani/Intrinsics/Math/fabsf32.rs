// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `fabsf32` returns the expected results: absolute value if argument
// is not NaN, otherwise NaN
#![feature(core_intrinsics)]

#[kani::proof]
fn test_abs_finite() {
    let x: f32 = kani::any();
    kani::assume(!x.is_nan());
    let abs_x = unsafe { std::intrinsics::fabsf32(x) };
    if x < 0.0 {
        assert!(-x == abs_x);
    } else {
        assert!(x == abs_x);
    }
}

#[kani::proof]
fn test_abs_nan() {
    let x: f32 = kani::any();
    kani::assume(x.is_nan());
    let abs_x = unsafe { std::intrinsics::fabsf32(x) };
    assert!(abs_x.is_nan());
}
