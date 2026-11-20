// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks

//! Copies, `write_bytes` and swaps write bytes, not values: Kani records the bytes they write
//! and checks the typed reads that cover them, c.f.
//! <https://github.com/model-checking/kani/issues/3762> and
//! <https://github.com/model-checking/kani/issues/5190>.

use std::mem::MaybeUninit;
const SURROGATE: u32 = 0xD800; // not a valid `char`

// --- must report UB
#[kani::proof]
fn copy_invalid_char() {
    let v: u32 = SURROGATE;
    let src = &v as *const u32 as *const char;
    let mut dst = 'a';
    unsafe { core::ptr::copy(src, &mut dst, 1) };
    let _c = dst;
}

#[kani::proof]
fn copy_nonoverlapping_invalid_char() {
    let v: u32 = SURROGATE;
    let src = &v as *const u32 as *const char;
    let mut dst = 'a';
    unsafe { core::ptr::copy_nonoverlapping(src, &mut dst, 1) };
    let _c = dst;
}

/// The issue's example: a write through `*mut u32` into a `char`, then a read.
unsafe fn copy_invalid_char_u32(ptr: &mut char) {
    let invalid_value: u32 = 0xDE01;
    unsafe { core::ptr::copy(&invalid_value, ptr as *mut _ as *mut u32, 1) };
}

#[kani::proof]
fn issue_check_with_ub() {
    let mut c = 'a';
    unsafe { copy_invalid_char_u32(&mut c) };
    assert_ne!(c, 'a');
}

#[kani::proof]
fn write_bytes_then_read() {
    let mut buf = [MaybeUninit::<char>::uninit(); 2];
    let val: u8 = kani::any();
    let p = buf.as_mut_ptr() as *mut char;
    unsafe { core::ptr::write_bytes(p, val, 1) };
    let _c = unsafe { *p }; // UB unless the bytes make a valid char
}

// --- must verify
#[kani::proof]
fn issue_check_no_ub() {
    let mut c = 'a';
    unsafe { copy_invalid_char_u32(&mut c) };
    c = 'b';
    assert_ne!(c, 'a');
}

#[kani::proof]
fn copy_u8() {
    let src = kani::any::<u8>();
    let mut dst = 0u8;
    unsafe { core::ptr::copy_nonoverlapping(&src, &mut dst, 1) };
    assert_eq!(dst, src);
}

#[kani::proof]
fn copy_nonoverlapping_char() {
    let src: char = kani::any();
    let mut dst: char = 'a';
    unsafe { std::ptr::copy_nonoverlapping(&src, &mut dst, 1) };
    assert!(dst == src);
}

#[kani::proof]
fn copy_from_slice_char() {
    let src: [char; 2] = kani::any();
    let mut dst: [char; 2] = ['a', 'b'];
    dst.copy_from_slice(&src);
    assert!(dst == src);
}

#[kani::proof]
fn swap_char() {
    let mut a: char = kani::any();
    let mut b: char = kani::any();
    let (a0, b0) = (a, b);
    std::mem::swap(&mut a, &mut b);
    assert!(a == b0 && b == a0);
}

#[kani::proof]
fn write_bytes_zero_count() {
    let mut buf = [MaybeUninit::<char>::uninit(); 2];
    let val: u8 = kani::any();
    unsafe { std::ptr::write_bytes(buf.as_mut_ptr() as *mut char, val, 0) };
}

#[kani::proof]
fn write_bytes_one() {
    let mut buf = [MaybeUninit::<char>::uninit(); 2];
    let val: u8 = kani::any();
    unsafe { std::ptr::write_bytes(buf.as_mut_ptr() as *mut char, val, 1) };
}

#[kani::proof]
fn option_string_through_ref() {
    let s: Option<String> = if kani::any() { Some(String::new()) } else { None };
    let r = &s;
    assert!(r.is_some() || r.is_none());
}

#[kani::proof]
fn vec_char_insert_remove() {
    let mut v: Vec<char> = Vec::with_capacity(4);
    v.push(kani::any());
    v.push(kani::any());
    v.insert(0, 'x'); // shifts with ptr::copy
    let c = v.remove(1);
    assert!(v.len() == 2 && (c as u32) <= 0x10FFFF);
}
