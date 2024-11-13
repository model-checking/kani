// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z mem-predicates -Z c-ffi
//! Check that Kani's memory predicates return that it's not safe to access pointers with foreign
//! types since it cannot compute its size.
#![feature(ptr_metadata)]
#![feature(extern_types)]

extern crate kani;

use kani::mem::{can_dereference, can_read_unaligned, can_write};
use std::ffi::c_void;
use std::ptr;

#[derive(Clone, Copy, kani::Arbitrary)]
struct Wrapper<T: ?Sized, U> {
    _size: U,
    _value: T,
}

extern "C" {
    type Foreign;
    type __CPROVER_size_t;
    fn __CPROVER_havoc_object(p: *mut c_void);

}

#[kani::proof]
pub fn check_write_is_unsafe() {
    let mut var: Wrapper<u64, usize> = kani::any();
    let fat_ptr: *mut Wrapper<Foreign, usize> = &mut var as *mut _ as *mut _;
    assert!(!can_write(fat_ptr));
}

#[kani::proof]
pub fn check_read_is_unsafe() {
    let var: usize = kani::any();
    let ptr = &var as *const _ as *const __CPROVER_size_t;
    assert!(!can_dereference(ptr));
    assert!(!can_read_unaligned(ptr));
}

/// Kani APIs cannot tell if that's safe to write to a foreign type.
///
/// However, foreign APIs that have knowledge of the type can still safely set new values.
#[kani::proof]
#[kani::should_panic]
pub fn check_write_with_extern() {
    let mut var = 0usize;
    let ptr = &mut var as *mut _ as *mut __CPROVER_size_t;
    unsafe {
        __CPROVER_havoc_object(ptr as *mut c_void);
    };
    assert!(var == 0);
}

/// Check that Kani can still build the foreign type using from_raw_parts.
#[kani::proof]
pub fn check_from_raw_parts() {
    let mut var: Wrapper<u64, usize> = kani::any();
    let ptr = &mut var as *mut _ as *mut ();
    let fat_ptr: *mut __CPROVER_size_t = ptr::from_raw_parts_mut(ptr, ());
    assert!(!can_write(fat_ptr));
}
