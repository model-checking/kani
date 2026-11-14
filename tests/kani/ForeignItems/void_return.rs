// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi --c-lib tests/kani/ForeignItems/void_fns.c

//! Check that C functions returning `void`, declared in Rust without a return type, link with
//! their C definitions and are called. Each harness asserts on the function's side effect, so
//! the call cannot be dropped.
//! See <https://github.com/model-checking/kani/issues/2151> and
//! <https://github.com/model-checking/kani/issues/1817>.

extern "C" {
    static mut COUNTER: u32;

    fn increment();
    fn set_counter(value: u32);
    fn add_into(out: *mut u32, a: u32, b: u32);
    fn swap(a: &mut u32, b: &mut u32);
    fn add_one(p: &mut u32);
    fn double_it(p: &mut u32);
}

/// The minimal case: no parameters, no result, a global side effect.
#[kani::proof]
fn check_void_no_args() {
    unsafe {
        assert_eq!(COUNTER, 0);
        increment();
        increment();
        assert_eq!(COUNTER, 2);
    }
}

/// A symbolic argument.
#[kani::proof]
fn check_void_with_arg() {
    let value: u32 = kani::any();
    unsafe {
        set_counter(value);
        assert_eq!(COUNTER, value);
    }
}

/// Several parameters, the result written through a raw pointer.
#[kani::proof]
fn check_void_out_param() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    let mut out: u32 = kani::any();
    unsafe { add_into(&mut out, a, b) };
    assert_eq!(out, a.wrapping_add(b));
}

/// Two pointer parameters.
#[kani::proof]
fn check_void_two_pointers() {
    let (x, y): (u32, u32) = (kani::any(), kani::any());
    let (mut a, mut b) = (x, y);
    unsafe { swap(&mut a, &mut b) };
    assert_eq!((a, b), (y, x));
}

/// A function pointer that may point to either of two `void` functions.
#[kani::proof]
fn check_void_fn_ptr_two_targets() {
    let before: u32 = kani::any_where(|v| *v < 1000);
    let use_add: bool = kani::any();
    let f: unsafe extern "C" fn(&mut u32) = if use_add { add_one } else { double_it };
    let mut x = before;
    unsafe { f(&mut x) };
    assert_eq!(x, if use_add { before + 1 } else { before * 2 });
}
