// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that we correctly generate invariants for enums.

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
        unsafe impl kani::Invariant for $enum_type {
            fn is_valid(&self) -> bool {
                matches!(*self, $enum_type::Variant1 | $enum_type::Variant2)
                    || matches!(*self, $enum_type::Variant3)
            }
        }

        fn $fn_name() {
            let e = kani::any::<$enum_type>();
            match e {
                $enum_type::Variant1 => {
                    let val = e as $repr;
                    assert!(val == $v1);
                    return;
                }
                $enum_type::Variant2 => {
                    let val = e as $repr;
                    assert!(val == $v2);
                    return;
                }
                $enum_type::Variant3 => {
                    let val = e as $repr;
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

#[kani::proof]
fn main() {
    check_basic();
    check_continuous();
    check_random();
}
