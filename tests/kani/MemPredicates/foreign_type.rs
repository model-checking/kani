// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi
//! We used to check that Kani's memory predicates return that it's not safe to access pointers
//! with foreign types since it cannot compute its size, but with the introduction of
//! sized_hierarchy compilation (expectedly) fails. So all we can check is values through extern
//! types.
#![feature(ptr_metadata)]
#![feature(extern_types)]
#![feature(sized_hierarchy)]

extern crate kani;

use std::ffi::c_void;
use std::marker::PointeeSized;

#[derive(Clone, Copy, kani::Arbitrary)]
struct Wrapper<T: PointeeSized, U> {
    _size: U,
    _value: T,
}

extern "C" {
    type __CPROVER_size_t;
    fn __CPROVER_havoc_object(p: *mut c_void);

}

/// Kani APIs cannot tell if that's safe to write to a foreign type.
///
/// However, foreign APIs that have knowledge of the type can still safely set new values.
#[kani::proof]
pub fn check_write_with_extern() {
    let mut var = 0usize;
    let ptr = &mut var as *mut _ as *mut __CPROVER_size_t;
    unsafe {
        __CPROVER_havoc_object(ptr as *mut c_void);
    };
}

/// Kani APIs cannot tell if that's safe to write to a foreign type.
///
/// However, foreign APIs that have knowledge of the type can still safely set new values, and
/// any side effect will be taken into consideration in the verification.
#[kani::proof]
#[kani::should_panic]
pub fn check_write_with_extern_side_effect() {
    let mut var = 0usize;
    let ptr = &mut var as *mut _ as *mut __CPROVER_size_t;
    unsafe {
        __CPROVER_havoc_object(ptr as *mut c_void);
    };
    assert!(var == 0);
}
