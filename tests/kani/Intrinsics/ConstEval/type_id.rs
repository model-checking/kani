// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that the `type_id` intrinsic is supported with common data types
// and that there are no duplicate type IDs
#![feature(core_intrinsics)]
use std::intrinsics::type_id;

struct MyStruct {}

enum MyEnum {}

#[kani::proof]
fn main() {
    let type_ids = [
        // Scalar types
        type_id::<i8>(),
        type_id::<i16>(),
        type_id::<i32>(),
        type_id::<i64>(),
        type_id::<i128>(),
        type_id::<isize>(),
        type_id::<u8>(),
        type_id::<u16>(),
        type_id::<u32>(),
        type_id::<u64>(),
        type_id::<u128>(),
        type_id::<usize>(),
        type_id::<f32>(),
        type_id::<f64>(),
        type_id::<bool>(),
        type_id::<char>(),
        // Compound types (tuple and array)
        type_id::<(i32, i32)>(),
        type_id::<[i32; 5]>(),
        // Custom data types (struct and enum)
        type_id::<MyStruct>(),
        type_id::<MyEnum>(),
    ];

    // Check that there are no duplicate type IDs
    let i: usize = kani::any();
    let j: usize = kani::any();
    kani::assume(i < type_ids.len());
    kani::assume(j < type_ids.len());
    if i != j {
        assert_ne!(type_ids[i], type_ids[j]);
    }
}
