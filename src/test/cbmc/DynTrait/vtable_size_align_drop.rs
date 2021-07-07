// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks the `size` and `align` fields of vtables, for a
// dynamic trait where two implementing structs have different sizes.
// The strategy is to use the new pointer metadata API:
// https://github.com/rust-lang/rust/issues/81513

#![feature(core_intrinsics)]
#![feature(ptr_metadata)]

use std::intrinsics::size_of;
use std::ptr::drop_in_place;

include!("../Helpers/vtable_utils_ignore.rs");
include!("../../rmc-prelude.rs");

// Different sized data fields on each struct
struct Sheep {
    pub sheep_num: i32,
}
struct Cow {
    pub cow_num: i8,
}

trait Animal {
    // Instance method signature
    fn noise(&self) -> i32;
}

// Implement the `Animal` trait for `Sheep`.
impl Animal for Sheep {
    fn noise(&self) -> i32 {
        self.sheep_num
    }
}

// Implement the `Animal` trait for `Cow`.
impl Animal for Cow {
    fn noise(&self) -> i32 {
        self.cow_num as i32
    }
}

// Returns some struct that implements Animal, but we don't know which one at compile time.
fn random_animal(random_number: i64) -> Box<dyn Animal> {
    if random_number < 5 { Box::new(Sheep { sheep_num: 7 }) } else { Box::new(Cow { cow_num: 9 }) }
}

fn main() {
    // The vtable is laid out as the right hand side here:
    //
    // +-------+------------------+
    // | size  |      value       |
    // +-------+------------------+
    // | usize | pointer to drop  |
    // | usize | size in bytes    |
    // | usize | align in bytes   |
    // | ?     | function ptrs... |
    // +-------+------------------+
    //
    // This layout taken from miri's handling:
    // https://github.com/rust-lang/rust/blob/ec487bf3cfc9ce386da25169509fae8f2b4d4eec/compiler/rustc_mir/src/interpret/traits.rs#L155

    // Check layout/values for Sheep
    unsafe {
        let animal_sheep = &*random_animal(1);

        // Check that the struct's data is what we expect
        let data_ptr = data!(animal_sheep);

        // Note: i32 ptr cast
        assert!(*(data_ptr as *mut i32) == 7); // From Sheep

        let vtable_ptr = vtable!(animal_sheep);

        // Drop pointer
        assert!(drop_from_vtable(vtable_ptr) == drop_in_place::<Sheep> as *mut ());

        // Size and align as usizes
        assert!(size_from_vtable(vtable_ptr) == size_of::<i32>());
        assert!(align_from_vtable(vtable_ptr) == size_of::<i32>());
    }
    // Check layout/values for Cow
    unsafe {
        let animal_cow = &*random_animal(6);

        // Check that the struct's data is what we expect
        let data_ptr = data!(animal_cow);

        // Note: i8 ptr cast
        assert!(*(data_ptr as *mut i8) == 9); // From Cow

        let vtable_ptr = vtable!(animal_cow);

        // Drop pointer
        assert!(drop_from_vtable(vtable_ptr) == drop_in_place::<Cow> as *mut ());

        // Size and align as usizes
        assert!(size_from_vtable(vtable_ptr) == size_of::<i8>());
        assert!(align_from_vtable(vtable_ptr) == size_of::<i8>());
    }
}
