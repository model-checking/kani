// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `copysignf64` returns a floating point value with the magnitude
// of `mag` and the sign of `sgn`. There are two special cases to consider:
//  * If `mag` is NaN, then NaN with the sign of `sgn` is returned.
//  * If `sgn` is -0, the result is only negative if the implementation supports
//    the signed zero consistenly in arithmetic operations.
#![feature(core_intrinsics)]

// NaN values in either `mag` or `sgn` can lead to spurious failures in these
// harnesses, so they are excluded when needed.
#[kani::proof]
fn test_copysign() {
    let mag: f64 = kani::any();
    let sig: f64 = kani::any();
    kani::assume(!mag.is_nan());
    kani::assume(!sig.is_nan());

    // Build the expected result
    let abs_mag = mag.abs();
    let expected_res = if sig.is_sign_positive() { abs_mag } else { -abs_mag };

    let res = unsafe { std::intrinsics::copysignf64(mag, sig) };

    // Compare against the expected result
    assert!(expected_res == res);
}

#[kani::proof]
fn test_copysign_mag_nan() {
    let mag: f64 = kani::any();
    let sig: f64 = kani::any();
    kani::assume(mag.is_nan());
    kani::assume(!sig.is_nan());

    let res = unsafe { std::intrinsics::copysignf64(mag, sig) };

    // Check the result is NaN with the expected sign
    if sig.is_sign_positive() {
        assert!(res.is_nan());
        assert!(res.is_sign_positive());
    } else {
        assert!(res.is_nan());
        assert!(res.is_sign_negative());
    }
}

#[kani::proof]
fn test_copysign_sig_neg_zero() {
    let mag: f64 = kani::any();
    let sig: f64 = -0.0;

    let res = unsafe { std::intrinsics::copysignf64(mag, sig) };

    // Check that the result is negative. This case is already included in the
    // general case, but we provide it for clarity.
    assert!(res.is_sign_negative());
}
