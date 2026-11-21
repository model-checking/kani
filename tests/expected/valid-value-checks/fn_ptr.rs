// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check the validity of function pointers, which must not be null
//! (<https://github.com/model-checking/kani/issues/3760>), alone, in an `Option` (whose `None` is
//! the null pointer), in structs and in enums with other variants.

use std::mem::transmute;

fn one() -> u8 {
    1
}

fn two() -> u8 {
    2
}

#[allow(dead_code)]
struct Callback {
    data: u8,
    f: fn() -> u8,
}

#[allow(dead_code)]
enum Action {
    Call(fn() -> u8),
    Value(u8),
}

#[kani::proof]
fn null_fn_ptr_should_fail() {
    let _f: fn() -> u8 = unsafe { transmute(0usize) };
}

#[kani::proof]
fn fn_ptr_valid() {
    let f: fn() -> u8 = if kani::any() { one } else { two };
    let addr: usize = unsafe { transmute(f) };
    let g: fn() -> u8 = unsafe { transmute(addr) };
    assert!(g() == f());
}

#[kani::proof]
fn option_fn_ptr_valid() {
    let addr: usize = if kani::any() { 0 } else { unsafe { transmute(one as fn() -> u8) } };
    let o: Option<fn() -> u8> = unsafe { transmute(addr) };
    assert!(o.is_none() == (addr == 0));
}

#[kani::proof]
fn read_null_fn_ptr_should_fail() {
    let addr = 0usize;
    let _f = unsafe { (&addr as *const usize as *const fn() -> u8).read() };
}

#[kani::proof]
fn struct_with_null_fn_ptr_should_fail() {
    let mut c = Callback { data: 0, f: one };
    unsafe { std::ptr::write_bytes(&mut c.f as *mut fn() -> u8 as *mut u8, 0, size_of::<usize>()) };
    let _c = unsafe { (&c as *const Callback).read() };
}

#[kani::proof]
fn enum_with_null_fn_ptr_should_fail() {
    let mut a = Action::Call(one);
    if let Action::Call(f) = &mut a {
        unsafe { (f as *mut fn() -> u8 as *mut usize).write(0) };
    }
    let _a = unsafe { (&a as *const Action).read() };
}

#[kani::proof]
fn enum_with_fn_ptr_valid() {
    let a = if kani::any() { Action::Call(two) } else { Action::Value(kani::any()) };
    let copy = unsafe { (&a as *const Action).read() };
    if let Action::Call(f) = copy {
        assert!(f() == 2);
    }
}

/// A function item is a zero-sized type, with no requirement: the requirements of a struct that
/// holds one are its other fields'.
struct WithFnItem<F> {
    f: F,
    flag: bool,
}

fn read_copy<T>(value: &T) -> T {
    unsafe { (value as *const T).read() }
}

#[kani::proof]
fn fn_item_field_valid() {
    let w = WithFnItem { f: one, flag: kani::any() };
    let copy = read_copy(&w);
    assert!((copy.f)() == 1 && copy.flag == w.flag);
}

#[kani::proof]
fn fn_item_field_invalid_flag_should_fail() {
    let mut w = WithFnItem { f: two, flag: true };
    // Call the function, so that Kani codegens it.
    assert!((w.f)() == 2);
    unsafe { (&mut w.flag as *mut bool as *mut u8).write(2) };
    let _copy = read_copy(&w);
}
