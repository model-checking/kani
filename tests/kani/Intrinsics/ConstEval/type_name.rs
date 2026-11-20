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
    const I8_NAME: &str = const { type_name::<i8>() };
    const I16_NAME: &str = const { type_name::<i16>() };
    const I32_NAME: &str = const { type_name::<i32>() };
    const I64_NAME: &str = const { type_name::<i64>() };
    const I128_NAME: &str = const { type_name::<i128>() };
    const ISIZE_NAME: &str = const { type_name::<isize>() };
    const U8_NAME: &str = const { type_name::<u8>() };
    const U16_NAME: &str = const { type_name::<u16>() };
    const U32_NAME: &str = const { type_name::<u32>() };
    const U64_NAME: &str = const { type_name::<u64>() };
    const U128_NAME: &str = const { type_name::<u128>() };
    const USIZE_NAME: &str = const { type_name::<usize>() };
    const F32_NAME: &str = const { type_name::<f32>() };
    const F64_NAME: &str = const { type_name::<f64>() };
    const BOOL_NAME: &str = const { type_name::<bool>() };
    const CHAR_NAME: &str = const { type_name::<char>() };
    // Compound types (tuple and array)
    const TUPLE_NAME: &str = const { type_name::<(i32, i32)>() };
    const ARRAY_NAME: &str = const { type_name::<[i32; 5]>() };
    // Custom data types (struct and enum)
    const STRUCT_NAME: &str = const { type_name::<MyStruct>() };
    const ENUM_NAME: &str = const { type_name::<MyEnum>() };

    assert_eq!(I8_NAME, "i8");
    assert_eq!(I16_NAME, "i16");
    assert_eq!(I32_NAME, "i32");
    assert_eq!(I64_NAME, "i64");
    assert_eq!(I128_NAME, "i128");
    assert_eq!(ISIZE_NAME, "isize");
    assert_eq!(U8_NAME, "u8");
    assert_eq!(U16_NAME, "u16");
    assert_eq!(U32_NAME, "u32");
    assert_eq!(U64_NAME, "u64");
    assert_eq!(U128_NAME, "u128");
    assert_eq!(USIZE_NAME, "usize");
    assert_eq!(F32_NAME, "f32");
    assert_eq!(F64_NAME, "f64");
    assert_eq!(BOOL_NAME, "bool");
    assert_eq!(CHAR_NAME, "char");
    assert_eq!(TUPLE_NAME, "(i32, i32)");
    assert_eq!(ARRAY_NAME, "[i32; 5]");
    assert_eq!(STRUCT_NAME, "type_name::MyStruct");
    assert_eq!(ENUM_NAME, "type_name::MyEnum");
}
