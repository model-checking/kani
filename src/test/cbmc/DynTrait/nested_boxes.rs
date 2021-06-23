// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks the size and align fields for 3-deep nested trait pointers. The
// outter 2 dynamic trait objects have fat pointers as their backing data.

#![feature(core_intrinsics)]
#![feature(raw)]
#![allow(deprecated)]

use std::intrinsics::size_of;
use std::mem::transmute;
use std::raw::TraitObject;

include!("../Helpers/vtable_utils_ignore.rs");

struct Foo {
    pub _a: i32,
    pub _b: i8,
}

fn main() {
    let dyn_trait1: Box<dyn Send> = Box::new(Foo { _a: 1, _b: 2 });
    let dyn_trait2: Box<dyn Send> = Box::new(dyn_trait1);
    let dyn_trait3: Box<dyn Send> = Box::new(dyn_trait2);

    // Do some unsafe magic to check that we generate the right three vtables
    unsafe {
        let trait_object3: TraitObject = transmute(dyn_trait3);

        // Outermost trait object
        // The size is 16, because the data is another fat pointer
        let vtable3: *mut usize = trait_object3.vtable as *mut usize;
        assert!(size_from_vtable(vtable3) == 16);
        assert!(align_from_vtable(vtable3) == 8);

        // Inspect the data pointer from dyn_trait3
        let data_ptr3 = trait_object3.data as *mut usize;

        // The second half of this fat pointer is another vtable, for dyn_trait2
        let vtable2 = *(data_ptr3.offset(1) as *mut *mut usize);

        // The size is 16, because the data is another fat pointer
        assert!(size_from_vtable(vtable2) == 16);
        assert!(align_from_vtable(vtable2) == 8);

        // Inspect the data pointer from dyn_trait2
        let data_ptr2 = *(data_ptr3 as *mut *mut usize);

        // The second half of this fat pointer is another vtable, for dyn_trait1
        let vtable1 = *(data_ptr2.offset(1) as *mut *mut usize);

        // The size is 8, because the data is the Foo itself
        assert!(size_from_vtable(vtable1) == size_of::<Foo>());
        assert!(align_from_vtable(vtable1) == 4);
    }
}
