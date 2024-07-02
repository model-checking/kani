// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::alloc::{alloc, alloc_zeroed, Layout};
use std::ptr;
use std::ptr::addr_of;
use std::slice::from_raw_parts;

#[repr(C)]
struct S(u32, u8);

#[kani::proof]
fn access_padding_init() {
    let s = S(0, 0);
    let ptr: *const u8 = addr_of!(s) as *const u8;
    let data = unsafe { *(ptr.add(3)) }; // Accessing data bytes is valid.
}

#[kani::proof]
fn alloc_to_slice() {
    let layout = Layout::from_size_align(32, 8).unwrap();
    unsafe {
        let ptr = alloc(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        *ptr.add(16) = 0x00;
        let val = *(ptr.add(2)); // Accessing previously initialized byte is valid.
    }
}

#[kani::proof]
fn alloc_zeroed_to_slice() {
    let layout = Layout::from_size_align(32, 8).unwrap();
    unsafe {
        // This returns initialized memory, so any further accesses are valid.
        let ptr = alloc_zeroed(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        *ptr.add(16) = 0x00;
        let slice1 = from_raw_parts(ptr, 16);
        let slice2 = from_raw_parts(ptr.add(16), 16);
    }
}

#[kani::proof]
fn struct_padding_and_arr_init() {
    unsafe {
        let mut s = S(0, 0);
        let sptr = ptr::addr_of_mut!(s);
        let sptr2 = sptr as *mut [u8; 4];
        *sptr2 = [0; 4];
        *sptr = S(0, 0);
        // Both S(u32, u8) and [u8; 4] have the same layout, so the memory is initialized.
        let val = *sptr2;
    }
}

#[kani::proof]
fn vec_read_init() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe { *v.as_mut_ptr().add(5) = 0x42 };
    let def = unsafe { *v.as_ptr().add(5) }; // Accessing previously initialized byte is valid.
    let x = def + 1;
}
