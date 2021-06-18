// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

macro_rules! test_floats {
    ($ty:ty) => {
        let a: $ty = __nondet();
        let b = a / 2.0;

        if a < 0.0 {
            assert!(a <= b);
        } else if a >= 0.0 {
            assert!(a >= b);
        }

        let c = b * 2.0;
        // general/infinity            Close but not exact                    NAN
        assert!(a == c || a - c < 0.00000001 || c - a < 0.00000001 || c * 0.0 != 0.0);

        let d: $ty = 0.0;
        assert!(d + 1.0 > 0.0);
        assert!(d - 1.0 < 0.0);
    };
}

fn main() {
    assert!(1.1 == 1.1 * 1.0);
    assert!(1.1 != 1.11 / 1.0);

    test_floats!(f32);
    test_floats!(f64);
}
