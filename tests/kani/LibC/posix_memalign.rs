// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Check support for `posix_memalign`.

#![feature(rustc_private)]
#![feature(allocator_api)]
extern crate libc;

use std::alloc::{Allocator, Layout, System};
use std::ptr;

#[repr(C, align(32))]
struct MyStruct {
    data: [u128; 10],
}

#[kani::proof]
fn alloc_zeroed() {
    let layout = Layout::new::<MyStruct>();
    let ptr = System.allocate_zeroed(layout).unwrap();
    assert_eq!(unsafe { ptr.as_ref()[0] }, 0);
}

// Source rust/src/libstd/sys/unix/alloc.rs
unsafe fn aligned_malloc(layout: &Layout) -> *mut u8 {
    let mut out = ptr::null_mut();
    let ret = libc::posix_memalign(&mut out, layout.align(), layout.size());
    if ret != 0 { ptr::null_mut() } else { out as *mut u8 }
}

#[kani::proof]
fn aligned_malloc_main() {
    let mut layout = Layout::from_size_align(0, 1);
    let _mem = unsafe { aligned_malloc(&layout.unwrap()) };
}

#[kani::proof]
fn posix_memalign_incorrect_alignment() {
    let mut out = ptr::null_mut();
    let small_page_size = 1;
    let size = 4;
    assert_eq!(unsafe { libc::posix_memalign(&mut out, small_page_size, size) }, libc::EINVAL);
    let incorrect_page_size = 13;
    assert_eq!(unsafe { libc::posix_memalign(&mut out, incorrect_page_size, size) }, libc::EINVAL);
}
