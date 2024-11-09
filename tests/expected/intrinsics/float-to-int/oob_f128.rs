// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(f128)]

// Check that `f128::MAX` does not fit in some integer types

macro_rules! check_cast {
    ($name:ident, $t:ty) => {
        #[kani::proof]
        fn $name() {
            let x = f128::MAX;
            let _u: $t = unsafe { x.to_int_unchecked() };
        }
    };
}

check_cast!(check_cast_u8, u8);
check_cast!(check_cast_u16, u16);
check_cast!(check_cast_u32, u32);
check_cast!(check_cast_u64, u64);
check_cast!(check_cast_u128, u128);
check_cast!(check_cast_usize, usize);

check_cast!(check_cast_i8, i8);
check_cast!(check_cast_i16, i16);
check_cast!(check_cast_i32, i32);
check_cast!(check_cast_i64, i64);
check_cast!(check_cast_i128, i128);
check_cast!(check_cast_isize, isize);
