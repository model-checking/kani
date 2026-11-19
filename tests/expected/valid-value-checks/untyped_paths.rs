// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks

//! Paths by which untyped bytes reach a typed read. Each `ub_*` harness is UB under Miri.

use std::num::NonZeroU8;
use std::ptr;

#[kani::proof]
#[inline(never)]
fn inner_proof() {}

/// A call to another proof function doesn't forget the untyped writes made before it.
#[kani::proof]
fn ub_after_nested_proof() {
    let mut b = true;
    unsafe { ptr::write_bytes(&raw mut b as *mut u8, 2, 1) };
    inner_proof();
    let x = b;
    std::hint::black_box(x);
}

/// A closure's captures can be written untyped; calling it reads them.
#[kani::proof]
fn ub_closure_capture() {
    let b = true;
    let mut f = move || b;
    unsafe { ptr::write_bytes(&raw mut f as *mut u8, 2, 1) };
    let r = f();
    std::hint::black_box(r);
}

/// A closure whose captures are only written typed.
#[kani::proof]
fn closure_capture_valid() {
    let b: bool = kani::any();
    let c: char = kani::any();
    let f = move || (b, c);
    assert_eq!(f(), (b, c));
}

/// A valid `NonZeroU8` isn't a valid `bool`.
#[kani::proof]
fn ub_store_of_other_restricted_type() {
    let mut b = true;
    let p = &raw mut b as *mut u8 as *mut NonZeroU8;
    unsafe { *p = NonZeroU8::new(2).unwrap() };
    let x = b;
    std::hint::black_box(x);
}

/// A volatile load of untyped bytes.
#[kani::proof]
fn ub_volatile_load() {
    let mut b = true;
    let p = &raw mut b;
    unsafe { ptr::write_bytes(p as *mut u8, 2, 1) };
    let x = unsafe { ptr::read_volatile(p) };
    std::hint::black_box(x);
}

/// A volatile store of a `u8` through a `bool`'s memory.
#[kani::proof]
fn ub_volatile_store() {
    let mut b = true;
    unsafe { ptr::write_volatile(&raw mut b as *mut u8, 2) };
    let x = b;
    std::hint::black_box(x);
}

/// A reference to untyped bytes, without a read: Kani requires the target of a reference to be
/// valid (UB under Miri's `-Zmiri-recursive-validation`).
#[kani::proof]
fn ub_reference() {
    let mut b = true;
    unsafe { ptr::write_bytes(&raw mut b as *mut u8, 2, 1) };
    let r = &b;
    std::hint::black_box(r as *const bool);
}

#[inline(never)]
fn make_ok() -> Result<bool, u8> {
    Ok(true)
}

/// A call result replaces untyped bytes with a valid value.
#[kani::proof]
fn call_result_overwrites() {
    let mut x: Result<bool, u8> = Err(0);
    unsafe { ptr::write_bytes(&raw mut x as *mut u8, 0xff, 2) };
    x = make_ok();
    assert!(x.is_ok());
}
