// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A struct whose last field is an `extern type` is unsized, but pointers to it are thin. Kani used
//! to treat them as fat pointers and crash. See https://github.com/model-checking/kani/issues/2256.
#![feature(extern_types)]

use std::ptr::addr_of;

unsafe extern "C" {
    type Opaque;
}

unsafe impl Sync for Opaque {}

#[repr(C)]
pub struct List<T> {
    len: usize,
    data: [T; 0],
    tail: Opaque,
}

#[repr(C)]
pub struct ListImpl<T, const N: usize> {
    len: usize,
    data: [T; N],
}

impl<T> List<T> {
    const fn as_slice(&self) -> &[T] {
        unsafe {
            let ptr = addr_of!(self.tail) as *const T;
            std::slice::from_raw_parts(ptr, self.len)
        }
    }
}

impl<T, const N: usize> ListImpl<T, N> {
    const fn as_list(&self) -> &List<T> {
        unsafe { std::mem::transmute(self) }
    }
}

pub static A: ListImpl<u128, 3> = ListImpl { len: 3, data: [5, 6, 7] };
pub static A_REF: &'static List<u128> = A.as_list();

#[kani::proof]
fn check_list() {
    assert_eq!(A_REF.as_slice(), &[5, 6, 7]);
}

#[kani::proof]
#[kani::should_panic]
fn check_list_wrong() {
    assert_eq!(A_REF.as_slice(), &[5, 6, 8]);
}

#[repr(C)]
struct Header {
    data: u32,
    tail: Opaque,
}

/// Dereferencing a raw pointer to such a struct checks the fields before the `extern type`.
#[kani::proof]
fn check_raw_pointer_deref() {
    let value: u32 = kani::any();
    let ptr = &value as *const u32 as *const Header;
    let r: &Header = unsafe { &*ptr };
    assert_eq!(r.data, value);
}
