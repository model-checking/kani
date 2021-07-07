// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Because each regression test does not share a crate, we just use
// an import! to share this code across test directories.

// Macro rules because we can't cast between incompatible dyn trait fat pointer types
macro_rules! data {
    ($f:ident) => {{
        unsafe {
            let ptr: *mut dyn std::any::Any = std::mem::transmute($f);
            let data: *mut () = ptr.cast();
            data
        }
    }};
}

macro_rules! vtable {
    ($f:ident) => {{
        unsafe {
            let ptr: *mut dyn std::any::Any = std::mem::transmute($f);
            std::ptr::metadata(ptr)
        }
    }};
}

fn drop_from_vtable(vtable_ptr: std::ptr::DynMetadata<dyn std::any::Any>) -> *mut () {
    // 1st pointer-sized position
    unsafe {
        let ptr: *mut usize = std::mem::transmute(vtable_ptr);
        *ptr as *mut ()
    }
}

fn size_from_vtable(vtable_ptr: std::ptr::DynMetadata<dyn std::any::Any>) -> usize {
    // 2nd usize-sized position
    vtable_ptr.size_of()
}

fn align_from_vtable(vtable_ptr: std::ptr::DynMetadata<dyn std::any::Any>) -> usize {
    // 3rd usize-sized position
    vtable_ptr.align_of()
}
