// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Transmuting a raw pointer to a reference to a different type used to crash the memory
//! initialization checks, which assumed both point to the same type. The reference's pointee is
//! what has to be initialized, so that is what gets checked.

use std::alloc::{Layout, alloc};

#[kani::proof]
fn check_to_smaller_type() {
    let x: u32 = 42;
    let ptr: *const u32 = &x;
    let r: &u8 = unsafe { std::mem::transmute(ptr) };
    assert!(*r == 42 || *r == 0);
}

/// Only the byte the `&u8` points to has to be initialized, not the whole `u32`.
#[kani::proof]
fn check_to_smaller_type_partly_init() {
    unsafe {
        let ptr = alloc(Layout::new::<u32>());
        *ptr = 7;
        let r: &u8 = std::mem::transmute(ptr as *const u32);
        assert_eq!(*r, 7);
    }
}

/// The same transmute from uninitialized memory is UB.
#[kani::proof]
fn check_to_smaller_type_uninit() {
    unsafe {
        let ptr = alloc(Layout::new::<u32>()) as *const u32;
        let _r: &u8 = std::mem::transmute(ptr);
    }
}

/// A `u16` is larger than the `u8` the pointer points to, so its second byte is not initialized.
#[kani::proof]
fn check_to_larger_type() {
    let x: u8 = 42;
    let ptr: *const u8 = &x;
    let _r: &u16 = unsafe { std::mem::transmute(ptr) };
}
