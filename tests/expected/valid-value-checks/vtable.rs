// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check that the metadata of a pointer to a trait object is a vtable for its trait
//! (<https://github.com/model-checking/kani/issues/3738>), for pointers built with
//! `ptr::from_raw_parts`, transmuted, or read from memory. The pointers are turned into references
//! with `transmute`, which checks them too.
#![feature(ptr_metadata)]

use std::ptr::{self, DynMetadata};

trait A {
    fn a(&self) -> u8;
}

trait B {
    fn b(&self) -> u8;
}

trait Sub: A {
    fn sub(&self) -> u8;
}

impl A for u8 {
    fn a(&self) -> u8 {
        *self
    }
}

impl B for u8 {
    fn b(&self) -> u8 {
        *self + 1
    }
}

impl Sub for u8 {
    fn sub(&self) -> u8 {
        *self + 2
    }
}

impl A for bool {
    fn a(&self) -> u8 {
        *self as u8
    }
}

#[kani::proof]
fn from_raw_parts_valid() {
    let x = 5u8;
    let r: &dyn A = &x;
    let (data, meta) = (r as *const dyn A).to_raw_parts();
    let p: *const dyn A = ptr::from_raw_parts(data, meta);
    let r: &dyn A = unsafe { std::mem::transmute(p) };
    assert!(r.a() == 5);
}

#[kani::proof]
fn from_raw_parts_forged_metadata_should_fail() {
    let x = 5u8;
    let meta: DynMetadata<dyn A> = unsafe { std::mem::transmute(kani::any::<usize>()) };
    let _p: *const dyn A = ptr::from_raw_parts(&x as *const u8 as *const (), meta);
}

#[kani::proof]
fn from_raw_parts_other_trait_should_fail() {
    // The vtable of `u8` for `B`, used as one for `A`.
    let x = 5u8;
    let meta_b = ptr::metadata(&x as &dyn B);
    let meta_a: DynMetadata<dyn A> = unsafe { std::mem::transmute(meta_b) };
    let _p: *const dyn A = ptr::from_raw_parts(&x as *const u8 as *const (), meta_a);
}

#[kani::proof]
fn upcast_metadata_valid() {
    let x = 5u8;
    let s: &dyn Sub = &x;
    let a: &dyn A = s;
    let meta = ptr::metadata(a);
    let p: *const dyn A = ptr::from_raw_parts(&x as *const u8 as *const (), meta);
    let r: &dyn A = unsafe { std::mem::transmute(p) };
    assert!(r.a() == 5);
}

#[kani::proof]
fn auto_trait_metadata_valid() {
    let x = 5u8;
    let s: &(dyn A + Send) = &x;
    let a: &dyn A = s;
    let p: *const dyn A = ptr::from_raw_parts(&x as *const u8 as *const (), ptr::metadata(a));
    let r: &dyn A = unsafe { std::mem::transmute(p) };
    assert!(r.a() == 5);
}

#[kani::proof]
fn transmute_forged_ref_should_fail() {
    let x = 5u8;
    let forged: usize = kani::any();
    let _r: &dyn A = unsafe { std::mem::transmute((&x as *const u8, forged)) };
}

#[kani::proof]
fn transmute_ref_valid() {
    let x = 5u8;
    let y = true;
    let r: &dyn A = if kani::any() { &x } else { &y };
    let parts: (usize, usize) = unsafe { std::mem::transmute(r) };
    let back: &dyn A = unsafe { std::mem::transmute(parts) };
    assert!(back.a() <= 5);
}

#[kani::proof]
fn read_raw_dyn_ptr_should_fail() {
    let x = 5u8;
    let parts: (usize, usize) = (&x as *const u8 as usize, 8);
    let _p = unsafe { (&parts as *const (usize, usize) as *const *const dyn A).read() };
}
