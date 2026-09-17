// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `size_of` intrinsic
// with common data types
//
// As of nightly-2026-08-01 `std::intrinsics::size_of` is a comptime fn and can only be called at
// compile time, so each call is bound to a `const` and the assertions compare those. That is
// exactly what this directory is about -- const evaluation of the intrinsic.
#![feature(core_intrinsics)]
use std::intrinsics::size_of;

struct MyStruct {}

enum MyEnum {}

#[kani::proof]
fn main() {
    // Scalar types
    const S1: usize = size_of::<i8>();
    assert!(S1 == 1);
    const S2: usize = size_of::<i16>();
    assert!(S2 == 2);
    const S3: usize = size_of::<i32>();
    assert!(S3 == 4);
    const S4: usize = size_of::<i64>();
    assert!(S4 == 8);
    const S5: usize = size_of::<i128>();
    assert!(S5 == 16);
    const S6: usize = size_of::<isize>();
    assert!(S6 == 8);
    const S7: usize = size_of::<u8>();
    assert!(S7 == 1);
    const S8: usize = size_of::<u16>();
    assert!(S8 == 2);
    const S9: usize = size_of::<u32>();
    assert!(S9 == 4);
    const S10: usize = size_of::<u64>();
    assert!(S10 == 8);
    const S11: usize = size_of::<u128>();
    assert!(S11 == 16);
    const S12: usize = size_of::<usize>();
    assert!(S12 == 8);
    const S13: usize = size_of::<f32>();
    assert!(S13 == 4);
    const S14: usize = size_of::<f64>();
    assert!(S14 == 8);
    const S15: usize = size_of::<bool>();
    assert!(S15 == 1);
    const S16: usize = size_of::<char>();
    assert!(S16 == 4);
    // Compound types (tuple and array)
    const S17: usize = size_of::<(i32, i32)>();
    assert!(S17 == 8);
    const S18: usize = size_of::<[i32; 5]>();
    assert!(S18 == 20);
    // Custom data types (struct and enum)
    const S19: usize = size_of::<MyStruct>();
    assert!(S19 == 0);
    const S20: usize = size_of::<MyEnum>();
    assert!(S20 == 0);
}
