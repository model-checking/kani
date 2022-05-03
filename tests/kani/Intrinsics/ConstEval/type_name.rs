// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `type_name` intrinsic
// with common data types
#![feature(core_intrinsics)]
use std::intrinsics::type_name;

struct MyStruct {}

enum MyEnum {}

#[kani::proof]
fn main() {
    // Scalar types
    assert!(type_name::<i8>() == "i8");
    assert!(type_name::<i16>() == "i16");
    assert!(type_name::<i32>() == "i32");
    assert!(type_name::<i64>() == "i64");
    assert!(type_name::<i128>() == "i128");
    assert!(type_name::<isize>() == "isize");
    assert!(type_name::<u8>() == "u8");
    assert!(type_name::<u16>() == "u16");
    assert!(type_name::<u32>() == "u32");
    assert!(type_name::<u64>() == "u64");
    assert!(type_name::<u128>() == "u128");
    assert!(type_name::<usize>() == "usize");
    assert!(type_name::<f32>() == "f32");
    assert!(type_name::<f64>() == "f64");
    assert!(type_name::<bool>() == "bool");
    assert!(type_name::<char>() == "char");
    // Compound types (tuple and array)
    assert!(type_name::<(i32, i32)>() == "(i32, i32)");
    assert!(type_name::<[i32; 5]>() == "[i32; 5]");
    // Custom data types (struct and enum)
    assert!(type_name::<MyStruct>() == "type_name::MyStruct");
    assert!(type_name::<MyEnum>() == "type_name::MyEnum");
}
