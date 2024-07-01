// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any and kani::any_raw can be used with standard floats i.e f32 and f64.

#![feature(f16)]
#![feature(f128)]

macro_rules! test_standard_floats {
    ( $type: ty ) => {{
        let v1 = kani::any::<$type>();
        let v2 = kani::any::<$type>();
        kani::cover!(v1 == v2, "This may be true");
        kani::cover!(v1 != v2, "This may also be true");
        kani::cover!(v1.is_nan(), "NaN should be valid float");
        kani::cover!(v1.is_subnormal(), "Subnormal should be valid float");
        kani::cover!(v1.is_normal(), "Normal should be valid float");
        kani::cover!(!v1.is_finite(), "Non-finite numbers are valid float");
    }};
}

#[kani::proof]
fn check_f32() {
    test_standard_floats!(f32);
}

#[kani::proof]
fn check_f64() {
    test_standard_floats!(f64);
}
