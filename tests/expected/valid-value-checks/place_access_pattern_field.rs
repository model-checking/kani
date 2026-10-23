// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Keep explicit NotNull field projections safe from unsupported stable type conversion.
#![feature(pattern_types, pattern_type_macro)]

// This is the same pattern kind used by NonNull's private pointer field.
type NotNull = std::pat::pattern_type!(*const u8 is !null);

#[repr(C)]
struct Holder {
    pointer: NotNull,
}

#[inline(never)]
unsafe fn field_address(ptr: *const Holder) -> *const NotNull {
    unsafe { &raw const (*ptr).pointer }
}

#[kani::proof]
fn not_null_field_projection() {
    let byte = 1u8;
    let pointer = &raw const byte;
    let holder = (&raw const pointer).cast::<Holder>();
    let field = unsafe { field_address(holder) };
    assert_eq!(field.cast::<*const u8>(), &raw const pointer);
    kani::cover!(true);
}

#[kani::proof]
fn raw_address_of_invalid_pattern_is_valid() {
    // These initialized bytes are not a valid NotNull, but forming their raw address is valid.
    let pointer: *const u8 = std::ptr::null();
    let holder = (&raw const pointer).cast::<Holder>();
    let field = unsafe { field_address(holder) };
    assert_eq!(field.cast::<*const u8>(), &raw const pointer);
    kani::cover!(true);
}
