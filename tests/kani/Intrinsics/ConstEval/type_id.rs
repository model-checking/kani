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
        const { type_id::<i8>() },
        const { type_id::<i16>() },
        const { type_id::<i32>() },
        const { type_id::<i64>() },
        const { type_id::<i128>() },
        const { type_id::<isize>() },
        const { type_id::<u8>() },
        const { type_id::<u16>() },
        const { type_id::<u32>() },
        const { type_id::<u64>() },
        const { type_id::<u128>() },
        const { type_id::<usize>() },
        const { type_id::<f32>() },
        const { type_id::<f64>() },
        const { type_id::<bool>() },
        const { type_id::<char>() },
        // Compound types (tuple and array)
        const { type_id::<(i32, i32)>() },
        const { type_id::<[i32; 5]>() },
        // Custom data types (struct and enum)
        const { type_id::<MyStruct>() },
        const { type_id::<MyEnum>() },
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
