// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any behaves correcty with NonZero types. This also shows how using the unsafe
// kind can generate UB.

use std::num::*;

macro_rules! test {
    ( $type: ty ) => {{
        let v1 = kani::any::<$type>();
        assert!(v1.get() != 0, "Any should not generate value zero");

        let option = Some(v1);
        assert!(option.is_some(), "Niche optimization works well.");

        let v2 = unsafe { kani::any_raw::<$type>() };
        kani::expect_fail(v2.get() != 0, "Any raw may generate invalid value.");

        let option = Some(v2);
        kani::expect_fail(option.is_some(), "Yep. This can happen due to niche optimization");
    }};
}

#[kani::proof]
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
