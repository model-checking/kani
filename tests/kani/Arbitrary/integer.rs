// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any can be used with integers.

macro_rules! harness {
    ( $fn_name: ident, $type: ty ) => {
        #[kani::proof]
        fn $fn_name() {
            let v1 = kani::any::<$type>();
            let v2 = kani::any::<$type>();
            kani::expect_fail(v1 == v2, "This may not be true");
            kani::expect_fail(v1 != v2, "This may also not be true");
            kani::expect_fail(v1 != <$type>::MAX, "v1 may be MAX");
            kani::expect_fail(v1 != <$type>::MIN, "v1 may be MIN");
        }
    };
}

harness!(check_i8, i8);
harness!(check_i16, i16);
harness!(check_i32, i32);
harness!(check_i64, i64);
harness!(check_i128, i128);
harness!(check_isize, isize);

harness!(check_u8, u8);
harness!(check_u16, u16);
harness!(check_u32, u32);
harness!(check_u64, u64);
harness!(check_u128, u128);
harness!(check_usize, usize);
