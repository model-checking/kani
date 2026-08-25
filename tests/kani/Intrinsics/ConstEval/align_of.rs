// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `align_of` intrinsic
// with common data types
//
// As of nightly-2026-08-01 `std::intrinsics::align_of` is a comptime fn and can only be called at
// compile time, so each call is bound to a `const` and the assertions compare those. That is
// exactly what this directory is about -- const evaluation of the intrinsic.
#![feature(core_intrinsics)]
use std::intrinsics::align_of;

struct MyStruct {}

enum MyEnum {}

#[kani::proof]
fn main() {
    // for the following types x86_64 and aarch64 agree on the alignment; see
    // AlignOfVal/align_of_fat_ptr.rs for some example of where they don't agree
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        // Scalar types
        const S1: usize = align_of::<i8>();
        assert!(S1 == 1);
        const S2: usize = align_of::<i16>();
        assert!(S2 == 2);
        const S3: usize = align_of::<i32>();
        assert!(S3 == 4);
        const S4: usize = align_of::<i64>();
        assert!(S4 == 8);
        const S5: usize = align_of::<i128>();
        assert!(S5 == 16);
        const S6: usize = align_of::<isize>();
        assert!(S6 == 8);
        const S7: usize = align_of::<u8>();
        assert!(S7 == 1);
        const S8: usize = align_of::<u16>();
        assert!(S8 == 2);
        const S9: usize = align_of::<u32>();
        assert!(S9 == 4);
        const S10: usize = align_of::<u64>();
        assert!(S10 == 8);
        const S11: usize = align_of::<u128>();
        assert!(S11 == 16);
        const S12: usize = align_of::<usize>();
        assert!(S12 == 8);
        const S13: usize = align_of::<f32>();
        assert!(S13 == 4);
        const S14: usize = align_of::<f64>();
        assert!(S14 == 8);
        const S15: usize = align_of::<bool>();
        assert!(S15 == 1);
        const S16: usize = align_of::<char>();
        assert!(S16 == 4);
        // Compound types (tuple and array)
        const S17: usize = align_of::<(i32, i32)>();
        assert!(S17 == 4);
        const S18: usize = align_of::<[i32; 5]>();
        assert!(S18 == 4);
        // Custom data types (struct and enum)
        const S19: usize = align_of::<MyStruct>();
        assert!(S19 == 1);
        const S20: usize = align_of::<MyEnum>();
        assert!(S20 == 1);
    }
}
