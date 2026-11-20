// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check value validity of niche-optimized enums whose layout is a single scalar
//! (`Option<NonNull<T>>`, `Option<&T>`, `Option<bool>`, ...), which used to be reported as
//! unsupported.

use std::ptr::NonNull;

#[kani::proof]
fn transmute_null_to_option_nonnull() {
    let p: *mut u8 = std::ptr::null_mut();
    let o: Option<NonNull<u8>> = unsafe { std::mem::transmute(p) };
    assert!(o.is_none());
}

#[kani::proof]
fn transmute_to_option_ref() {
    let x = 7u8;
    let addr: usize = if kani::any() { &x as *const u8 as usize } else { 0 };
    let o: Option<&u8> = unsafe { std::mem::transmute(addr) };
    assert!(o.is_none() || *o.unwrap() == 7);
}

#[kani::proof]
fn read_valid_option_bool() {
    let byte: u8 = kani::any();
    kani::assume(byte <= 2);
    let o: Option<bool> = unsafe { *(&byte as *const u8 as *const Option<bool>) };
    assert_eq!(o.is_none(), byte == 2);
}

/// Negative control: the byte value 5 is not a valid `Option<bool>`.
#[kani::proof]
fn read_invalid_option_bool_should_fail() {
    let byte: u8 = 5;
    let _o: Option<bool> = unsafe { *(&byte as *const u8 as *const Option<bool>) };
}

/// `Option<char>` remains unsupported: its validity is not a single range (it has the
/// surrogate gap), which the enum's scalar `valid_range` cannot express.
#[kani::proof]
fn read_option_char_unsupported() {
    let v: u32 = 0x41;
    let _o: Option<char> = unsafe { *(&v as *const u32 as *const Option<char>) };
}
