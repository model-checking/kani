// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `size_of` intrinsic
// with common data types
#![feature(core_intrinsics)]
use std::intrinsics::size_of;

struct MyStruct {}

enum MyEnum {}

#[kani::proof]
fn main() {
    // Scalar types
    assert!(size_of::<i8>() == 1);
    assert!(size_of::<i16>() == 2);
    assert!(size_of::<i32>() == 4);
    assert!(size_of::<i64>() == 8);
    assert!(size_of::<i128>() == 16);
    assert!(size_of::<isize>() == 8);
    assert!(size_of::<u8>() == 1);
    assert!(size_of::<u16>() == 2);
    assert!(size_of::<u32>() == 4);
    assert!(size_of::<u64>() == 8);
    assert!(size_of::<u128>() == 16);
    assert!(size_of::<usize>() == 8);
    assert!(size_of::<f32>() == 4);
    assert!(size_of::<f64>() == 8);
    assert!(size_of::<bool>() == 1);
    assert!(size_of::<char>() == 4);
    // Compound types (tuple and array)
    assert!(size_of::<(i32, i32)>() == 8);
    assert!(size_of::<[i32; 5]>() == 20);
    // Custom data types (struct and enum)
    assert!(size_of::<MyStruct>() == 0);
    assert!(size_of::<MyEnum>() == 0);
}
