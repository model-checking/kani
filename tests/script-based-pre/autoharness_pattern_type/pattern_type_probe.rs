// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// `NonNull<T>` wraps a `pattern_type!(*const T is !null)` (since nightly-2026-04-01).
// Autoharness must be able to derive Arbitrary for this pattern type so that
// functions taking NonNull arguments can be verified.
use std::ptr::NonNull;

// Top-level NonNull argument: the generated value must be non-null.
pub fn nonnull_as_ptr(p: NonNull<u8>) -> *mut u8 {
    p.as_ptr()
}

// NonNull inside a struct: the pattern type appears as an ADT field.
pub struct Wrapper {
    inner: NonNull<u32>,
    tag: u8,
}

pub fn wrapper_get_ptr(w: Wrapper) -> *mut u32 {
    w.inner.as_ptr()
}

// Cover check: the generated NonNull must actually be non-null.
pub fn nonnull_is_not_null(p: NonNull<u8>) {
    kani::cover!(p.as_ptr() as usize != 0, "non-null pointer");
}
