// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that rmc::any behaves correcty with NonZero types. This also shows how using the unsafe
// kind can generate UB.

use std::num::*;

macro_rules! test {
    ( $type: ty ) => {{
        let v1 = rmc::any::<$type>();
        assert!(v1.get() != 0, "Any should not generate value zero");

        let option = Some(v1);
        assert!(option.is_some(), "Niche optimization works well.");

        let v2 = unsafe { rmc::any_raw::<$type>() };
        rmc::expect_fail(v2.get() != 0, "Any raw may generate invalid value.");

        let option = Some(v2);
        rmc::expect_fail(option.is_some(), "Yep. This can happen due to niche optimization");
    }};
}

fn main() {
    test!(NonZeroI8);
    test!(NonZeroI16);
    test!(NonZeroI32);
    test!(NonZeroI64);
    test!(NonZeroI128);
    test!(NonZeroIsize);

    test!(NonZeroU8);
    test!(NonZeroU16);
    test!(NonZeroU32);
    test!(NonZeroU64);
    test!(NonZeroU128);
    test!(NonZeroUsize);
}
