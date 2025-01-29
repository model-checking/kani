// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Disable undefined function checks because of a failure
// https://github.com/model-checking/kani/issues/555
// kani-flags: --no-undefined-function-checks

#![feature(core_intrinsics)]
#![feature(ptr_metadata)]

use std::any::Any;
use std::io::{Write, sink};
use std::ptr::DynMetadata;

include!("../Helpers/vtable_utils_ignore.rs");

#[kani::proof]
fn main() {
    let mut log: Box<dyn Write + Send> = Box::new(sink());
    let dest: Box<dyn Write + Send> = Box::new(log.as_mut());

    let mut log2: Box<dyn Write + Send> = Box::new(sink());
    let buffer = vec![1, 2, 3, 5, 8];
    let num_bytes = log2.write(&buffer).unwrap();
    assert!(num_bytes == 5);

    // Do some unsafe magic to check that we generate the right two vtables
    unsafe {
        // The vtable has [&drop, size, align, ....]
        let dest_ptr = &*dest;
        let dest_vtable_ptr = vtable!(dest_ptr);

        // The size is 16, because the data is another fat pointer
        assert!(size_from_vtable(dest_vtable_ptr) == 16);
        assert!(align_from_vtable(dest_vtable_ptr) == 8);

        // Inspect the data pointer from dest
        let dest_data_ptr = data!(dest_ptr) as *mut usize;

        // // The second half of this fat pointer is another vtable, for log
        let second_vtable_ptr = dest_data_ptr.offset(1) as *mut DynMetadata<dyn Any>;
        let second_vtable = *second_vtable_ptr;

        // The sink itself has no size, weirdly enough
        assert!(size_from_vtable(second_vtable) == 0);
        assert!(align_from_vtable(second_vtable) == 1);
    }
}
