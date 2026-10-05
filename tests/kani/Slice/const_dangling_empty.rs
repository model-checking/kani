// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that a constant empty slice or `&str` whose data pointer is dangling, and so has no
//! provenance, points to that address. This used to trigger an ICE reported in
//! <https://github.com/model-checking/kani/issues/3021>.

use std::ptr::NonNull;
use std::slice::from_raw_parts;

const PTR_U16: *const u16 = NonNull::dangling().as_ptr();
const CONST_U16_REF: &[u16] = unsafe { from_raw_parts(PTR_U16, 0) };
const EMPTY_STR: &str =
    unsafe { std::str::from_utf8_unchecked(from_raw_parts(NonNull::<u8>::dangling().as_ptr(), 0)) };

#[kani::proof]
fn main() {
    let ptr_u16 = unsafe { from_raw_parts(PTR_U16, 0) }.as_ptr();
    assert_eq!(ptr_u16, CONST_U16_REF.as_ptr());
    assert!(CONST_U16_REF.is_empty());
}

#[kani::proof]
fn check_empty_str() {
    assert_eq!(EMPTY_STR.as_ptr(), NonNull::<u8>::dangling().as_ptr() as *const u8);
    assert!(EMPTY_STR.is_empty());
}
