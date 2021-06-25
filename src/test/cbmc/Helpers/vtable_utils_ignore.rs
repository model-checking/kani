// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Because each regression test does not share a crate, we just use
// an import! to share this code across test directories.

// Macro rules because we can't cast between incompatible dyn trait fat pointer types
macro_rules! vtable {
    ($f:ident) => {{
        unsafe {
            let trait_object: std::raw::TraitObject = std::mem::transmute($f);
            trait_object.vtable as *mut usize
        }
    }};
}

macro_rules! data {
    ($f:ident) => {{
        unsafe {
            let trait_object: std::raw::TraitObject = std::mem::transmute($f);
            trait_object.data as *mut ()
        }
    }};
}

fn drop_from_vtable(vtable_ptr: *mut usize) -> *mut () {
    // 1st pointer-sized position
    unsafe { *vtable_ptr as *mut () }
}

fn size_from_vtable(vtable_ptr: *mut usize) -> usize {
    // 2nd usize-sized position
    unsafe { *(vtable_ptr.offset(1)) }
}

fn align_from_vtable(vtable_ptr: *mut usize) -> usize {
    // 3rd usize-sized position
    unsafe { *(vtable_ptr.offset(2)) }
}
