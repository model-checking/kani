// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can correctly stub integer types functions.

/// Generate stub and harness for count_ones method on integers.
macro_rules! stub_count_ones {
    ($ty:ty, $harness:ident, $stub:ident) => {
        // Stub that always returns 0.
        pub fn $stub(_: $ty) -> u32 {
            0
        }

        // Harness
        #[kani::proof]
        #[kani::stub($ty::count_ones, $stub)]
        pub fn $harness() {
            let input = kani::any();
            let ones = <$ty>::count_ones(input);
            assert_eq!(ones, 0);
        }
    };
}

stub_count_ones!(u8, u8_count_ones, stub_u8_count_ones);
stub_count_ones!(u16, u16_count_ones, stub_u16_count_ones);
stub_count_ones!(u32, u32_count_ones, stub_u32_count_ones);
stub_count_ones!(u64, u64_count_ones, stub_u64_count_ones);
stub_count_ones!(u128, u128_count_ones, stub_u128_count_ones);
stub_count_ones!(usize, usize_count_ones, stub_usize_count_ones);

stub_count_ones!(i8, i8_count_ones, stub_i8_count_ones);
stub_count_ones!(i16, i16_count_ones, stub_i16_count_ones);
stub_count_ones!(i32, i32_count_ones, stub_i32_count_ones);
stub_count_ones!(i64, i64_count_ones, stub_i64_count_ones);
stub_count_ones!(i128, i128_count_ones, stub_i128_count_ones);
stub_count_ones!(isize, isize_count_ones, stub_isize_count_ones);
