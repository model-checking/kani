// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure we fail verification if the user tries to compute the size of a foreign item.
//!
//! Although it is currently safe to call `size_of_val` and `align_of_val` on foreign types,
//! the behavior is not well-defined.
//!
//! For now, this will trigger a safety check failure.
//!
//! From <https://doc.rust-lang.org/std/mem/fn.size_of_val_raw.html>:
//! > an (unstable) extern type, then this function is always safe to call,
//! > but may panic or otherwise return the wrong value, as the extern type’s layout is not known.

#![feature(extern_types, layout_for_ptr)]

extern "C" {
    type ExternalType;
}

#[kani::proof]
#[kani::should_panic]
fn check_size_of_foreign() {
    let tup: (u32, usize) = kani::any();
    assert_eq!(std::mem::size_of_val(&tup), 16);

    let ptr: *const (u32, ExternalType) = &tup as *const _ as *const _;
    let size = unsafe { std::mem::size_of_val_raw(ptr) };
    assert_eq!(size, 4);
}

#[kani::proof]
#[kani::should_panic]
fn check_align_of_foreign() {
    let tup: (u32, usize) = kani::any();
    assert_eq!(std::mem::align_of_val(&tup), 8);

    let ptr: *const (u32, ExternalType) = &tup as *const _ as *const _;
    let align = unsafe { std::mem::align_of_val_raw(ptr) };
    assert_eq!(align, 4);
}
