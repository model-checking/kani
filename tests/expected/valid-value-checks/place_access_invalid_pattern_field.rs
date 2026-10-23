// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Precise access to a pattern field must preserve its validity check before reading.
#![feature(pattern_types, pattern_type_macro)]

type NotNull = std::pat::pattern_type!(*const u8 is !null);

#[repr(C)]
struct Holder {
    pointer: NotNull,
}

#[inline(never)]
unsafe fn read_pointer(ptr: *const Holder) -> *const u8 {
    unsafe { std::mem::transmute((*ptr).pointer) }
}

#[kani::proof]
fn invalid_not_null_field_read() {
    // Initialized bytes representing null, not uninitialized memory.
    let pointer: *const u8 = std::ptr::null();
    let holder = (&raw const pointer).cast::<Holder>();
    let result = unsafe { read_pointer(holder) };
    assert!(result.is_null());
}
