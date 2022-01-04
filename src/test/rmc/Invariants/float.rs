// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// rmc-verify-fail
// Ensure that rmc::any and rmc::any_raw can be used with integers.

macro_rules! test {
    ( $type: ty ) => {{
        let v1 = rmc::any::<$type>();
        let v2 = unsafe { rmc::any_raw::<$type>() };
        rmc::expect_fail(v1 == v2, "This may not be true");
        rmc::expect_fail(v1 != v2, "This may also not be true");
        rmc::expect_fail(!v1.is_nan(), "NaN should be valid float");
        rmc::expect_fail(!v1.is_subnormal(), "Subnormal should be valid float");
        rmc::expect_fail(!v1.is_normal(), "Normal should be valid float");
        rmc::expect_fail(v1.is_finite(), "Non-finite numbers are valid float");
    }};
}

fn main() {
    test!(f32);
    test!(f64);
}
