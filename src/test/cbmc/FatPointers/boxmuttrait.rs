// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
#![feature(raw)]
#![allow(deprecated)]

use std::mem::transmute;
use std::raw::TraitObject;
use std::io::{sink, Write};

fn main() {
    let mut log: Box<dyn Write + Send> = Box::new(sink());
    let dest: Box<dyn Write + Send> = Box::new(log.as_mut());

    let mut log2: Box<dyn Write + Send> = Box::new(sink());
    let buffer = vec![1, 2, 3, 5, 8];
    let num_bytes = log2.write(&buffer).unwrap();
    assert!(num_bytes == 5);


    // Do some unsafe magic to check that we generate the right two vtables
    unsafe {
        let dest_trait_object: TraitObject = transmute(&*dest);
        
        // The vtable has [&drop, size, align, ....]
        // The first 3 values are pointers, so we can grab them with offset
        let dest_vtable_ptr = dest_trait_object.vtable as *mut usize;
        let dest_size_ptr = dest_vtable_ptr.offset(1) as *mut usize;
        let dest_align_ptr = dest_vtable_ptr.offset(2) as *mut usize;
        
        // The size is 16, because the data is another fat pointer
        assert!(*dest_size_ptr == 16);
        assert!(*dest_align_ptr == 8);
        
        // Inspect the data pointer from dest
        let dest_data_ptr = dest_trait_object.data as *mut usize;
        
        // // The second half of this fat pointer is another vtable, for log
        let second_vtable_ptr = dest_data_ptr.offset(1) as *mut *mut usize;
        let second_vtable = *second_vtable_ptr;
        let log_size_ptr = second_vtable.offset(1) as *mut usize;
        let log_align_ptr = second_vtable.offset(2) as *mut usize;

        // The sink itself has no size, weirdly enough
        assert!(*log_size_ptr == 0);
        assert!(*log_align_ptr == 1);
    }
}
