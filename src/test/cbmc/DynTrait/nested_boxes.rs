// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(core_intrinsics)]
#![feature(raw)]
#![allow(deprecated)]

use std::fs::File;
use std::intrinsics::size_of;
use std::mem::transmute;
use std::raw::TraitObject;

struct Foo {
    pub _a: i32,
    pub _b: i8,
}

fn main() {
    let dyn_trait1 : Box<dyn Send> = Box::new(Foo{ _a : 1, _b : 2});
    let dyn_trait2 : Box<dyn Send> = Box::new(dyn_trait1);
    let dyn_trait3 : Box<dyn Send> = Box::new(dyn_trait2);

    // Do some unsafe magic to check that we generate the right three vtables
    unsafe {
        let trait_object3: TraitObject = transmute(dyn_trait3);
        
        // Outermost trait object
        // The vtable has [&drop, size, align, ....]
        // The first 3 values are pointers, so we can grab them with offset
        let vtable_ptr3 = trait_object3.vtable as *mut usize;
        let size_ptr3 = vtable_ptr3.offset(1) as *mut usize;
        let align_ptr3 = vtable_ptr3.offset(2) as *mut usize;
        
        // The size is 16, because the data is another fat pointer
        assert!(*size_ptr3 == 16);
        assert!(*align_ptr3 == 8);
        
        // Inspect the data pointer from dyn_trait3
        let data_ptr3 = trait_object3.data as *mut usize;
        
        // The second half of this fat pointer is another vtable, for dyn_trait2
        let vtable_ptr2 = *(data_ptr3.offset(1) as *mut *mut usize);
        let size_ptr2 = vtable_ptr2.offset(1) as *mut usize;
        let align_ptr2 = vtable_ptr2.offset(2) as *mut usize;

        // // The size is 16, because the data is another fat pointer
        assert!(*size_ptr2 == 16);
        assert!(*align_ptr2 == 8);
        
        // Inspect the data pointer from dyn_trait2
        let data_ptr2 = *(data_ptr3 as *mut *mut usize);
        
        // The second half of this fat pointer is another vtable, for dyn_trait1
        let vtable_ptr1 = *(data_ptr2.offset(1) as *mut *mut usize);
        let size_ptr1 = vtable_ptr1.offset(1) as *mut usize;
        let align_ptr1 = vtable_ptr1.offset(2) as *mut usize;

        // // The size is 8, because the data is the Foo itself
        assert!(*size_ptr1 == size_of::<Foo>());
        assert!(*align_ptr1 == 4);
    }
}
