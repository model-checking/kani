// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Validity must follow the consumed field, not its uninitialized enclosing object.
//! Each independent harness has an end-of-path cover.

use std::mem::{self, MaybeUninit};

#[repr(C)]
struct Pair {
    initialized: bool,
    pending: bool,
}

#[kani::proof]
fn box_initialization() {
    let value: bool = kani::any();
    let boxed = Box::new(Pair { initialized: value, pending: false });
    assert_eq!(boxed.initialized, value);
    // Keep this test independent of the unsupported mutable casts in deallocation.
    mem::forget(boxed);
    kani::cover!(true);
}

#[kani::proof]
fn write_initialization() {
    let mut storage = MaybeUninit::<bool>::uninit();
    let value: bool = kani::any();
    unsafe {
        storage.as_mut_ptr().write(value);
        assert_eq!(storage.assume_init(), value);
    }
    kani::cover!(true);
}

#[kani::proof]
fn write_over_invalid_old_bytes() {
    // Unlike uninitialized storage, this deterministically exposes any check of the old bool.
    let mut byte: u8 = 2;
    let ptr = (&raw mut byte).cast::<bool>();
    unsafe {
        ptr.write(true);
        assert!(*ptr);
    }
    kani::cover!(true);
}

#[kani::proof]
fn raw_mut_field_address() {
    let mut storage = MaybeUninit::<Pair>::uninit();
    let ptr = storage.as_mut_ptr();
    let field = unsafe { &raw mut (*ptr).initialized };
    unsafe { field.write(true) };
    assert!(unsafe { *field });
    kani::cover!(true);
}

#[kani::proof]
fn raw_const_field_address() {
    let storage = MaybeUninit::<Pair>::uninit();
    let ptr = storage.as_ptr();
    let field = unsafe { &raw const (*ptr).pending };
    assert!(!field.is_null());
    kani::cover!(true);
}

#[kani::proof]
fn read_initialized_field() {
    let mut storage = MaybeUninit::<Pair>::uninit();
    let ptr = storage.as_mut_ptr();
    unsafe {
        (&raw mut (*ptr).initialized).write(true);
        assert!((*ptr).initialized);
    }
    kani::cover!(true);
}

#[kani::proof]
fn borrow_initialized_field() {
    let mut storage = MaybeUninit::<Pair>::uninit();
    let ptr = storage.as_mut_ptr();
    unsafe {
        (&raw mut (*ptr).initialized).write(true);
        let field = &(*ptr).initialized;
        assert!(*field);
    }
    kani::cover!(true);
}

#[kani::proof]
fn mutably_borrow_initialized_field() {
    let mut storage = MaybeUninit::<Pair>::uninit();
    let ptr = storage.as_mut_ptr();
    unsafe {
        (&raw mut (*ptr).initialized).write(true);
        let field = &mut (*ptr).initialized;
        *field = false;
        assert!(!*field);
    }
    kani::cover!(true);
}

#[kani::proof]
fn nested_pointer_store() {
    let mut storage = MaybeUninit::<bool>::uninit();
    let mut inner = storage.as_mut_ptr();
    let outer = &raw mut inner;
    unsafe {
        **outer = true;
        assert!(storage.assume_init());
    }
    kani::cover!(true);
}

#[kani::proof]
fn tuple_field_read() {
    let mut storage = MaybeUninit::<(bool, bool)>::uninit();
    let ptr = storage.as_mut_ptr();
    unsafe {
        (&raw mut (*ptr).0).write(true);
        assert!((*ptr).0);
    }
    kani::cover!(true);
}

#[inline(never)]
fn produce_bool() -> bool {
    true
}

#[kani::proof]
fn call_destination_initialization() {
    let mut storage = MaybeUninit::<bool>::uninit();
    unsafe {
        *storage.as_mut_ptr() = produce_bool();
        assert!(storage.assume_init());
    }
    kani::cover!(true);
}
