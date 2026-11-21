// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check the validity of references and `Box`
//! (<https://github.com/model-checking/kani/issues/3759>): they must be non-null and aligned, the
//! length of a slice they point to must keep its size under `isize::MAX`, and the metadata of a
//! pointer to a trait object must be a vtable for its trait. This covers `&Box<T>` and `Box<T>`
//! read from memory, and `Box<dyn Trait>`, which used to be reported as unsupported.

use std::mem::{ManuallyDrop, transmute};

trait Answer {
    fn answer(&self) -> u32;
}

impl Answer for u32 {
    fn answer(&self) -> u32 {
        *self
    }
}

impl Answer for bool {
    fn answer(&self) -> u32 {
        *self as u32
    }
}

#[kani::proof]
fn read_ref_box_valid() {
    let b = Box::new(5u8);
    let r = &b;
    let copy = unsafe { (&r as *const &Box<u8>).read() };
    assert!(**copy == 5);
}

#[kani::proof]
fn read_ref_box_after_copy_valid() {
    let b = Box::new(5u8);
    let r = &b;
    let mut dst = std::mem::MaybeUninit::<&Box<u8>>::uninit();
    unsafe { std::ptr::copy_nonoverlapping(&r, dst.as_mut_ptr(), 1) };
    let copy = unsafe { dst.assume_init() };
    assert!(**copy == 5);
}

#[kani::proof]
fn read_null_ref_box_should_fail() {
    let addr = 0usize;
    let _r = unsafe { (&addr as *const usize as *const &Box<u8>).read() };
}

#[kani::proof]
fn read_null_box_should_fail() {
    let addr = 0usize;
    let b = unsafe { (&addr as *const usize as *const ManuallyDrop<Box<u8>>).read() };
    let _ = b;
}

#[kani::proof]
fn misaligned_ref_should_fail() {
    let buf = [0u32; 2];
    let addr = (&buf as *const [u32; 2] as usize) + 1;
    let _r: &u32 = unsafe { transmute(addr) };
}

#[kani::proof]
fn misaligned_box_should_fail() {
    let buf = [0u64; 2];
    let addr = (&buf as *const [u64; 2] as usize)
        + kani::any_where(|off: &usize| *off % 8 != 0 && *off < 8);
    let b: ManuallyDrop<Box<u64>> = unsafe { transmute(addr) };
    let _ = b;
}

#[kani::proof]
fn aligned_ref_valid() {
    let buf = [kani::any::<u32>(); 2];
    let idx: usize = kani::any_where(|idx| *idx < 2);
    let addr = &buf[idx] as *const u32 as usize;
    let r: &u32 = unsafe { transmute(addr) };
    assert!(*r == buf[0]);
}

#[kani::proof]
fn slice_ref_too_long_should_fail() {
    let buf = [0u16; 4];
    let len: usize = kani::any_where(|len| *len > isize::MAX as usize / 2);
    let _s: &[u16] = unsafe { transmute((&buf as *const [u16; 4], len)) };
}

#[kani::proof]
fn slice_ref_valid() {
    let buf = [1u16; 4];
    let len: usize = kani::any_where(|len| *len <= 4);
    let s: &[u16] = unsafe { transmute((&buf as *const [u16; 4], len)) };
    assert!(s.len() == len);
}

#[kani::proof]
fn box_dyn_valid() {
    let b: Box<dyn Answer> = if kani::any() { Box::new(7u32) } else { Box::new(true) };
    let r = &b;
    let copy = unsafe { (&r as *const &Box<dyn Answer>).read() };
    assert!(copy.answer() == 7 || copy.answer() == 1);
}

#[kani::proof]
fn box_dyn_read_valid() {
    let b: Box<dyn Answer> = Box::new(7u32);
    let copy = unsafe { (&b as *const Box<dyn Answer>).read() };
    std::mem::forget(b);
    assert!(copy.answer() == 7);
}

#[kani::proof]
fn box_dyn_forged_vtable_should_fail() {
    let b: Box<dyn Answer> = Box::new(7u32);
    let (data, _vtable): (usize, usize) = unsafe { transmute(b) };
    let forged: usize = kani::any();
    let _b: ManuallyDrop<Box<dyn Answer>> = unsafe { transmute((data, forged)) };
}

#[kani::proof]
fn box_dyn_invalid_value_should_fail() {
    // A `Box<dyn Answer>` whose vtable is `bool`'s, pointing to the byte `2`.
    let b: Box<dyn Answer> = Box::new(true);
    let (data, vtable): (*mut u8, usize) = unsafe { transmute(b) };
    unsafe { *data = 2 };
    let b: Box<dyn Answer> = unsafe { transmute((data, vtable)) };
    let _ = b.answer();
}
