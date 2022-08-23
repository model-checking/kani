// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Check that user can correctly generate arbitrary types for enums.
//! TODO: We should replace these expect_fail / assert with cover statements.
//! <https://github.com/model-checking/kani/issues/696>
#[derive(Copy, Clone)]
enum Basic {
    Variant1,
    Variant2,
    Variant3,
}

#[derive(Copy, Clone)]
enum Continuous {
    Variant1 = 10,
    Variant2,
    Variant3,
}

#[derive(Copy, Clone)]
enum Random {
    Variant1 = -10,
    Variant2 = 100,
    Variant3 = 0,
}

macro_rules! check_enum {
    ( $fn_name:ident, $repr:ty, $enum_type:ident, $v1:literal, $v2:literal, $v3:literal ) => {
        impl kani::Arbitrary for $enum_type {
            fn any() -> Self {
                match kani::any() {
                    0 => $enum_type::Variant1,
                    1 => $enum_type::Variant2,
                    _ => $enum_type::Variant3,
                }
            }
        }

        #[kani::proof]
        fn $fn_name() {
            let e = kani::any::<$enum_type>();
            match e {
                $enum_type::Variant1 => {
                    let val = e as $repr;
                    kani::expect_fail(false, "This should be reachable");
                    assert!(val == $v1);
                    return;
                }
                $enum_type::Variant2 => {
                    let val = e as $repr;
                    kani::expect_fail(false, "This should be reachable");
                    assert!(val == $v2);
                    return;
                }
                $enum_type::Variant3 => {
                    let val = e as $repr;
                    kani::expect_fail(false, "This should be reachable");
                    assert!(val == $v3);
                    return;
                }
            }
        }
    };
}

check_enum!(check_basic, u8, Basic, 0, 1, 2);
check_enum!(check_continuous, u8, Continuous, 10, 11, 12);
check_enum!(check_random, i8, Random, -10, 100, 0);
