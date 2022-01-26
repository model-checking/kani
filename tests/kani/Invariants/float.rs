// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any and kani::any_raw can be used with integers.

macro_rules! test {
    ( $type: ty ) => {{
        let v1 = kani::any::<$type>();
        let v2 = unsafe { kani::any_raw::<$type>() };
        kani::expect_fail(v1 == v2, "This may not be true");
        kani::expect_fail(v1 != v2, "This may also not be true");
        kani::expect_fail(!v1.is_nan(), "NaN should be valid float");
        kani::expect_fail(!v1.is_subnormal(), "Subnormal should be valid float");
        kani::expect_fail(!v1.is_normal(), "Normal should be valid float");
        kani::expect_fail(v1.is_finite(), "Non-finite numbers are valid float");
    }};
}

fn main() {
    test!(f32);
    test!(f64);
}
