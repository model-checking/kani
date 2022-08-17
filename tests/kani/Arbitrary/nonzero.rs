// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure that kani::any behaves correcty with NonZero types.

use std::num::*;

macro_rules! harness {
    ( $fn_name: ident, $type: ty ) => {
        #[kani::proof]
        fn $fn_name() {
            let v1 = kani::any::<$type>();
            assert!(v1.get() != 0, "Any should not generate value zero");

            let option = Some(v1);
            assert!(option.is_some(), "Niche optimization works well.");
        }
    };
}

harness!(non_zero_i8, NonZeroI8);
harness!(non_zero_i16, NonZeroI16);
harness!(non_zero_i32, NonZeroI32);
harness!(non_zero_i64, NonZeroI64);
harness!(non_zero_i128, NonZeroI128);
harness!(non_zero_isize, NonZeroIsize);

harness!(non_zero_u8, NonZeroU8);
harness!(non_zero_u16, NonZeroU16);
harness!(non_zero_u32, NonZeroU32);
harness!(non_zero_u64, NonZeroU64);
harness!(non_zero_u128, NonZeroU128);
harness!(non_zero_usize, NonZeroUsize);
