// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that the `type_id` intrinsic is supported with common data types
#![feature(core_intrinsics)]
use std::intrinsics::type_id;

struct MyStruct {}

enum MyEnum {}

#[kani::proof]
fn main() {
    // Scalar types
    let _ = type_id::<i8>();
    let _ = type_id::<i16>();
    let _ = type_id::<i32>();
    let _ = type_id::<i64>();
    let _ = type_id::<i128>();
    let _ = type_id::<isize>();
    let _ = type_id::<u8>();
    let _ = type_id::<u16>();
    let _ = type_id::<u32>();
    let _ = type_id::<u64>();
    let _ = type_id::<u128>();
    let _ = type_id::<usize>();
    let _ = type_id::<f32>();
    let _ = type_id::<f64>();
    let _ = type_id::<bool>();
    let _ = type_id::<char>();
    // Compound types (tuple and array)
    let _ = type_id::<(i32, i32)>();
    let _ = type_id::<[i32; 5]>();
    // Custom data types (struct and enum)
    let _ = type_id::<MyStruct>();
    let _ = type_id::<MyEnum>();
}
