// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `align_of` intrinsic
// with common data types
#![feature(core_intrinsics)]
use std::intrinsics::align_of;

struct MyStruct {}

enum MyEnum {}

#[kani::proof]
fn main() {
    #[cfg(target_arch = "x86_64")]
    {
        // Scalar types
        assert!(align_of::<i8>() == 1);
        assert!(align_of::<i16>() == 2);
        assert!(align_of::<i32>() == 4);
        assert!(align_of::<i64>() == 8);
        assert!(align_of::<i128>() == 16);
        assert!(align_of::<isize>() == 8);
        assert!(align_of::<u8>() == 1);
        assert!(align_of::<u16>() == 2);
        assert!(align_of::<u32>() == 4);
        assert!(align_of::<u64>() == 8);
        assert!(align_of::<u128>() == 16);
        assert!(align_of::<usize>() == 8);
        assert!(align_of::<f32>() == 4);
        assert!(align_of::<f64>() == 8);
        assert!(align_of::<bool>() == 1);
        assert!(align_of::<char>() == 4);
        // Compound types (tuple and array)
        assert!(align_of::<(i32, i32)>() == 4);
        assert!(align_of::<[i32; 5]>() == 4);
        // Custom data types (struct and enum)
        assert!(align_of::<MyStruct>() == 1);
        assert!(align_of::<MyEnum>() == 1);
    }

    #[cfg(target_arch = "aarch64")]
    {
        // Scalar types
        assert!(align_of::<i8>() == 1);
        assert!(align_of::<i16>() == 2);
        assert!(align_of::<i32>() == 4);
        assert!(align_of::<i64>() == 8);
        assert!(align_of::<i128>() == 16);
        assert!(align_of::<isize>() == 8);
        assert!(align_of::<u8>() == 1);
        assert!(align_of::<u16>() == 2);
        assert!(align_of::<u32>() == 4);
        assert!(align_of::<u64>() == 8);
        assert!(align_of::<u128>() == 16);
        assert!(align_of::<usize>() == 8);
        assert!(align_of::<f32>() == 4);
        assert!(align_of::<f64>() == 8);
        assert!(align_of::<bool>() == 1);
        assert!(align_of::<char>() == 4);
        // Compound types (tuple and array)
        assert!(align_of::<(i32, i32)>() == 4);
        assert!(align_of::<[i32; 5]>() == 4);
        // Custom data types (struct and enum)
        assert!(align_of::<MyStruct>() == 1);
        assert!(align_of::<MyEnum>() == 1);
    }
}
