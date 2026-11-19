// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A reference must be non-null even when there is nothing behind it to read, as for an
//! `extern type`, whose size Kani takes to be 0. rustc's own null check skips unsized pointees.
#![feature(extern_types)]

unsafe extern "C" {
    type Opaque;
}

#[kani::proof]
fn check_null_ref() {
    let ptr: *const Opaque = std::ptr::null();
    let _r: &Opaque = unsafe { &*ptr };
}

/// A raw pointer to the same place may be null.
#[kani::proof]
fn check_null_raw_ptr() {
    let ptr: *const Opaque = std::ptr::null();
    let _p: *const Opaque = unsafe { &raw const *ptr };
}
