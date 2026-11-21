// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check dereferences of pointers to trait objects
//! (<https://github.com/model-checking/kani/issues/5215>), which used to be reported as
//! unsupported. The trait object must be valid at its dynamic type, the type its vtable was
//! generated for, as the target of a reference must be valid for a sized type. The trait object
//! at the tail of a struct (`RcInner<dyn Trait>`) is checked at its dynamically aligned offset.
#![feature(ptr_metadata)]
#![feature(downcast_unchecked)]

use std::any::Any;
use std::ptr;

trait A {
    fn a(&self) -> u8;
}

impl A for u8 {
    fn a(&self) -> u8 {
        *self
    }
}

impl A for bool {
    fn a(&self) -> u8 {
        *self as u8
    }
}

impl A for char {
    fn a(&self) -> u8 {
        *self as u8
    }
}

impl A for u64 {
    fn a(&self) -> u8 {
        *self as u8
    }
}

/// The reproducer of the issue.
#[kani::proof]
fn downcast_box_dyn_any() {
    let b: Box<dyn Any> = Box::new(kani::any::<i32>());
    let _ = b.downcast::<i32>();
}

#[kani::proof]
fn downcast_box_dyn_any_valid() {
    let v: i32 = kani::any();
    let b: Box<dyn Any> = if kani::any() { Box::new(v) } else { Box::new(true) };
    match b.downcast::<i32>() {
        Ok(i) => assert!(*i == v),
        Err(other) => assert!(other.is::<bool>()),
    }
}

#[kani::proof]
fn downcast_invalid_value_should_fail() {
    // A `Box<dyn Any>` that holds a `bool` whose byte is `2`: invalid when the `dyn Any` is
    // dereferenced to get its type, and when the `bool` is read after the downcast.
    let mut b: Box<dyn Any> = Box::new(true);
    let data = &mut *b as *mut dyn Any as *mut u8;
    unsafe { *data = 2 };
    if let Ok(v) = b.downcast::<bool>() {
        let _read = *v;
    }
}

#[kani::proof]
fn downcast_unchecked_valid() {
    // `downcast_unchecked` casts the `*mut dyn Any + Send + Sync` to a `*mut bool`.
    let v: bool = kani::any();
    let b: Box<dyn Any + Send + Sync> = Box::new(v);
    let d = unsafe { b.downcast_unchecked::<bool>() };
    assert!(*d == v);
}

#[kani::proof]
fn downcast_unchecked_invalid_value_should_fail() {
    let mut b: Box<dyn Any + Send + Sync> = Box::new(true);
    let data = &mut *b as *mut (dyn Any + Send + Sync) as *mut u8;
    unsafe { *data = 2 };
    let d = unsafe { b.downcast_unchecked::<bool>() };
    let _read = *d;
}

#[kani::proof]
fn write_through_cast_of_dyn_should_fail() {
    // A `u32` written over the `char` behind a `dyn A`, through a cast `*mut dyn A` that a store
    // of a type with validity requirements doesn't make unsupported: reading the `char` reports
    // it.
    let mut c: char = 'a';
    let p: *mut dyn A = &mut c as &mut dyn A;
    unsafe { *(p as *mut std::num::NonZeroU32) = std::num::NonZeroU32::new(0xD800).unwrap() };
    let _read = c;
}

#[kani::proof]
fn method_call_through_ref() {
    let x: u8 = kani::any();
    let y: bool = kani::any();
    let r: &dyn A = if kani::any() { &x } else { &y };
    assert!(r.a() == x || r.a() == y as u8);
}

#[kani::proof]
fn method_call_through_raw_ptr() {
    let x: u64 = kani::any();
    let p: *const dyn A = &x as &dyn A;
    assert!(unsafe { (*p).a() } == x as u8);
}

#[kani::proof]
fn method_call_through_from_raw_parts() {
    let x = 5u8;
    let meta = ptr::metadata(&x as &dyn A);
    let p: *const dyn A = ptr::from_raw_parts(&x as *const u8 as *const (), meta);
    assert!(unsafe { (*p).a() } == 5);
}

#[kani::proof]
fn deref_invalid_dyn_value_should_fail() {
    // Metadata of `bool`, data the byte `2`.
    let y = true;
    let meta = ptr::metadata(&y as &dyn A);
    let x = 2u8;
    let p: *const dyn A = ptr::from_raw_parts(&x as *const u8 as *const (), meta);
    let _r: &dyn A = unsafe { &*p };
}

#[kani::proof]
fn deref_dyn_value_valid() {
    let y = true;
    let meta = ptr::metadata(&y as &dyn A);
    let x: u8 = kani::any_where(|x| *x < 2);
    let p: *const dyn A = ptr::from_raw_parts(&x as *const u8 as *const (), meta);
    let r: &dyn A = unsafe { &*p };
    assert!(r.a() == x);
}

#[kani::proof]
fn rc_dyn_valid() {
    // `RcInner<dyn A>` has the trait object at its tail, after the counters.
    let r: std::rc::Rc<dyn A> =
        if kani::any() { std::rc::Rc::new(3u8) } else { std::rc::Rc::new(7u64) };
    let r2 = r.clone();
    assert!(r2.a() == 3 || r2.a() == 7);
}

#[kani::proof]
fn rc_dyn_invalid_value_should_fail() {
    let r: std::rc::Rc<dyn A> = std::rc::Rc::new(true);
    let data = std::rc::Rc::as_ptr(&r) as *const u8 as *mut u8;
    unsafe { *data = 2 };
    let _ = r.a();
}
