// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(core_intrinsics)]
#![feature(custom_mir)]
#![allow(dropping_copy_types)]

use std::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use std::intrinsics::mir::*;
use std::ptr;
use std::ptr::addr_of;
use std::slice::from_raw_parts;

#[repr(C)]
struct S(u32, u8);

#[kani::proof]
fn access_padding_init() {
    let s = S(0, 0);
    let ptr: *const u8 = addr_of!(s) as *const u8;
    let padding = unsafe { *(ptr.add(4)) };
}

#[kani::proof]
fn alloc_to_slice() {
    let layout = Layout::from_size_align(32, 8).unwrap();
    unsafe {
        // This returns initialized memory.
        let ptr = alloc(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        *ptr.add(16) = 0x00;
        let val = *(ptr.add(2));
    }
}

#[kani::proof]
fn alloc_zeroed_to_slice() {
    let layout = Layout::from_size_align(32, 8).unwrap();
    unsafe {
        // This returns initialized memory.
        let ptr = alloc_zeroed(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        *ptr.add(16) = 0x00;
        let slice1 = from_raw_parts(ptr, 16);
        let slice2 = from_raw_parts(ptr.add(16), 16);
        drop(slice1.cmp(slice2));
        dealloc(ptr, layout);
    }
}

#[kani::proof]
#[custom_mir(dialect = "runtime", phase = "optimized")]
fn struct_padding_and_arr_init() {
    mir! {
        let s: S;
        let sptr;
        let sptr2;
        let _val;
        {
            sptr = ptr::addr_of_mut!(s);
            sptr2 = sptr as *mut [u8; 4];
            *sptr2 = [0; 4];
            *sptr = S(0, 0);
            // Both S(u16, u16) and [u8; 4] have the same layout, so the memory is initialized.
            _val = *sptr2;
            Return()
        }
    }
}

#[kani::proof]
fn vec_read_init() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe { *v.as_mut_ptr().add(5) = 0x42 };
    let def = unsafe { *v.as_ptr().add(5) }; // Not UB since accessing initialized memory.
    let x = def + 1;
}
