// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `fabsf16` returns the expected results: absolute value if argument
// is not NaN, otherwise NaN
#![feature(core_intrinsics)]
#![feature(f16)]

#[kani::proof]
fn test_abs_finite() {
    let x: f16 = kani::any();
    kani::assume(!x.is_nan());
    let abs_x = unsafe { std::intrinsics::fabsf16(x) };
    if x < 0.0 {
        assert!(-x == abs_x);
    } else {
        assert!(x == abs_x);
    }
}

#[kani::proof]
fn test_abs_nan() {
    let x: f16 = kani::any();
    kani::assume(x.is_nan());
    let abs_x = unsafe { std::intrinsics::fabsf16(x) };
    assert!(abs_x.is_nan());
}
