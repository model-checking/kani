// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

//! Check that Kani flags a conversion of an out-of-bounds float value to an int
//! via `float_to_int_unchecked`

macro_rules! check_unsigned_lower {
    ($name:ident, $t:ty) => {
        #[kani::proof]
        fn $name() {
            let x: f32 = kani::any_where(|v: &f32| v.is_finite() && *v <= -1.0);
            let _u: $t = unsafe { std::intrinsics::float_to_int_unchecked(x) };
        }
    };
}

check_unsigned_lower!(check_u8_lower, u8);
check_unsigned_lower!(check_u16_lower, u16);
check_unsigned_lower!(check_u32_lower, u32);
check_unsigned_lower!(check_u64_lower, u64);
check_unsigned_lower!(check_u128_lower, u128);
check_unsigned_lower!(check_usize_lower, usize);

macro_rules! check_unsigned_upper {
    ($name:ident, $t:ty, $v:expr) => {
        #[kani::proof]
        fn $name() {
            let x: f32 = kani::any_where(|v: &f32| v.is_finite() && *v >= $v);
            let _u: $t = unsafe { std::intrinsics::float_to_int_unchecked(x) };
        }
    };
}

check_unsigned_upper!(check_u8_upper, u8, (1u128 << 8) as f32);
check_unsigned_upper!(check_u16_upper, u16, (1u128 << 16) as f32);
check_unsigned_upper!(check_u32_upper, u32, (1u128 << 32) as f32);
check_unsigned_upper!(check_u64_upper, u64, (1u128 << 64) as f32);
// this harness should pass
check_unsigned_upper!(check_u128_upper, u128, f32::INFINITY);
check_unsigned_upper!(check_usize_upper, usize, (1u128 << 64) as f32);
