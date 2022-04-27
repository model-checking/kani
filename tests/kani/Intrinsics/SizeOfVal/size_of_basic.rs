// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `size_of_val` intrinsic with
// common data types. Note that these tests assume an x86_64 architecture, which
// is the only architecture supported by Kani at the moment.
#![feature(core_intrinsics)]
use std::intrinsics::size_of_val;

struct MyStruct {
    val: u32,
}

#[repr(C)]
struct CStruct {
    a: u8,
    b: i32,
}

enum MyEnum {
    Variant,
}

#[kani::proof]
fn main() {
    unsafe {
        // Scalar types
        assert!(size_of_val(&0i8) == 1);
        assert!(size_of_val(&0i16) == 2);
        assert!(size_of_val(&0i32) == 4);
        assert!(size_of_val(&0i64) == 8);
        assert!(size_of_val(&0i128) == 16);
        assert!(size_of_val(&0isize) == 8);
        assert!(size_of_val(&0u8) == 1);
        assert!(size_of_val(&0u16) == 2);
        assert!(size_of_val(&0u32) == 4);
        assert!(size_of_val(&0u64) == 8);
        assert!(size_of_val(&0u128) == 16);
        assert!(size_of_val(&0usize) == 8);
        assert!(size_of_val(&0f32) == 4);
        assert!(size_of_val(&0f64) == 8);
        assert!(size_of_val(&false) == 1);
        assert!(size_of_val(&(0 as char)) == 4);
        // Compound types (tuple and array)
        assert!(size_of_val(&(0i32, 0i32)) == 8);
        assert!(size_of_val(&[0i32; 5]) == 20);
        // Custom data types (struct and enum)
        assert!(size_of_val(&MyStruct { val: 0u32 }) == 4);
        assert!(size_of_val(&MyEnum::Variant) == 0);
        assert!(size_of_val(&CStruct { a: 0u8, b: 0i32 }) == 8);
    }
}
