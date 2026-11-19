// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Invalid reads and borrows must retain checks for the consumed bool.

// Initialized bytes with an invalid bool representation, not uninitialized memory.

#[inline(never)]
unsafe fn read_bool(ptr: *const bool) -> bool {
    unsafe { *ptr }
}

#[kani::proof]
fn invalid_bool_read() {
    let byte: u8 = 2;
    let ptr = (&raw const byte).cast::<bool>();
    let value = unsafe { read_bool(ptr) };
    kani::cover!(value);
}

// Narrowing a check to a field must still detect that field's invalid representation.

#[repr(C)]
struct Pair {
    first: bool,
    second: bool,
}

#[inline(never)]
unsafe fn read_second(ptr: *const Pair) -> bool {
    unsafe { (*ptr).second }
}

#[kani::proof]
fn invalid_bool_field_read() {
    let bytes = [1u8, 2];
    let ptr = (&raw const bytes).cast::<Pair>();
    let value = unsafe { read_second(ptr) };
    kani::cover!(value);
}

// Only create a reference and observe its address; do not read the bool through it.

#[inline(never)]
unsafe fn borrow_shared_bool<'a>(ptr: *const bool) -> &'a bool {
    unsafe { &*ptr }
}

#[kani::proof]
fn invalid_bool_shared_borrow() {
    let byte: u8 = 2;
    let ptr = (&raw const byte).cast::<bool>();
    let reference = unsafe { borrow_shared_bool(ptr) };
    assert!(!std::ptr::from_ref(reference).is_null());
}

// Regression for incorrectly skipping all PlaceContext::MUTATING accesses.

#[inline(never)]
unsafe fn borrow_mut_bool<'a>(ptr: *mut bool) -> &'a mut bool {
    unsafe { &mut *ptr }
}

#[kani::proof]
fn invalid_bool_mut_borrow() {
    let mut byte: u8 = 2;
    // u8 has no restricted bit patterns: this cast must not mask the missing borrow check
    // with a mutable-cast UnsupportedCheck.
    let ptr = (&raw mut byte).cast::<bool>();
    let reference = unsafe { borrow_mut_bool(ptr) };
    assert!(!std::ptr::from_mut(reference).is_null());
}

#[repr(C)]
struct Inner {
    valid: bool,
    pending: bool,
}

#[repr(C)]
struct Outer {
    inner: Inner,
    other: bool,
}

#[inline(never)]
unsafe fn borrow_nested<'a>(ptr: *mut Outer) -> &'a mut bool {
    unsafe { &mut (*ptr).inner.pending }
}

#[kani::proof]
fn invalid_nested_mut_borrow() {
    // repr(C) puts the invalid bool at byte offset one; all bytes are initialized.
    let mut bytes = [1u8, 2, 1];
    let ptr = (&raw mut bytes).cast::<Outer>();
    let reference = unsafe { borrow_nested(ptr) };
    assert!(!std::ptr::from_mut(reference).is_null());
}
