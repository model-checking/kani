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
    }};
}

#[kani::proof]
fn main() {
    test!(i8);
    test!(i16);
    test!(i32);
    test!(i64);
    test!(i128);
    test!(isize);

    test!(u8);
    test!(u16);
    test!(u32);
    test!(u64);
    test!(u128);
    test!(usize);
}
