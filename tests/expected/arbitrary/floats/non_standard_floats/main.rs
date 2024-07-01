// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any and kani::any_raw can be used with non-standard floats i.e f16 and f128.

#![feature(f16)]
#![feature(f128)]

macro_rules! test_non_standard_floats {
    ( $type: ty ) => {{
        let v1 = kani::any::<$type>();
        let v2 = kani::any::<$type>();
        kani::cover!(v1 == v2, "This may be true");
        kani::cover!(v1 != v2, "This may also be true");
        kani::cover!(v1.is_nan(), "NaN should be valid float");
    }};
}

#[kani::proof]
fn check_f16() {
    test_non_standard_floats!(f16);
}

#[kani::proof]
fn check_f128() {
    test_non_standard_floats!(f128);
}
