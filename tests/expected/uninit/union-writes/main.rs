// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Writes of values that contain unions under `-Z uninit-checks`: the union bytes take the state
//! of the value written, whatever the operation, and the other bytes are marked as for any write
//! of the type. The `_ok` harnesses must verify; the `_ub` harnesses read uninitialized memory
//! and must be reported.

#![feature(core_intrinsics)]

use std::mem::MaybeUninit;
use std::ptr;

/// A swap moves the state of each value to the other place.
#[kani::proof]
fn swap_moves_uninit_ub() {
    let mut a = MaybeUninit::<u32>::new(5);
    let mut b = MaybeUninit::<u32>::uninit();
    unsafe { std::intrinsics::typed_swap_nonoverlapping(&raw mut a, &raw mut b) };
    let _ = unsafe { a.assume_init() };
}

#[kani::proof]
fn swap_moves_init_ok() {
    let mut a = MaybeUninit::<u32>::uninit();
    let mut b = MaybeUninit::<u32>::new(7);
    unsafe { std::intrinsics::typed_swap_nonoverlapping(&raw mut a, &raw mut b) };
    assert_eq!(unsafe { a.assume_init() }, 7);
}

/// A volatile store writes the value's state, uninitialized here.
#[kani::proof]
fn volatile_store_uninit_ub() {
    let mut a = MaybeUninit::<u32>::new(5);
    unsafe { ptr::write_volatile(&raw mut a, MaybeUninit::uninit()) };
    let _ = unsafe { a.assume_init() };
}

#[kani::proof]
fn volatile_store_init_ok() {
    let mut a = MaybeUninit::<u32>::uninit();
    unsafe { ptr::write_volatile(&raw mut a, MaybeUninit::new(9)) };
    assert_eq!(unsafe { a.assume_init() }, 9);
}

/// A struct with a union field and a padding byte.
#[repr(C)]
#[derive(Clone, Copy)]
struct Padded {
    a: u8,
    // 1 padding byte
    u: MaybeUninit<u16>,
}

/// Writing a value through a pointer leaves its padding uninitialized, even when the source's
/// bytes, padding included, were all initialized.
#[kani::proof]
fn padding_after_write_ub() {
    let mut src: Padded = unsafe { std::mem::zeroed() };
    src.a = 1;
    let mut dst = Padded { a: 0, u: MaybeUninit::new(0) };
    let p = &raw mut dst;
    unsafe { *p = src };
    let _padding = unsafe { *(p as *const u8).add(1) };
}

/// A union that holds a raw pointer to uninitialized memory, read as a reference.
union PtrOrRef {
    p: *const u8,
    r: &'static u8,
}

#[kani::proof]
fn union_to_reference_uninit_ub() {
    let byte = Box::leak(Box::new(MaybeUninit::<u8>::uninit()));
    let u = PtrOrRef { p: byte.as_ptr() };
    let r: &u8 = unsafe { std::mem::transmute(u) };
    let _ = *r;
}

#[kani::proof]
fn union_to_reference_init_ok() {
    let byte: &'static u8 = Box::leak(Box::new(4u8));
    let u = PtrOrRef { p: byte as *const u8 };
    let r: &u8 = unsafe { std::mem::transmute(u) };
    assert_eq!(*r, 4);
}

/// `MaybeUninit<&T>` only ever holds a valid reference.
#[kani::proof]
fn maybe_uninit_reference_ok() {
    let x = 3u8;
    let m = MaybeUninit::<&u8>::new(&x);
    assert_eq!(*unsafe { m.assume_init() }, 3);
}

/// Constants containing unions: their bytes' state is known.
const UNINIT: MaybeUninit<u32> = MaybeUninit::uninit();
const HALF: [MaybeUninit<u8>; 2] = [MaybeUninit::new(1), MaybeUninit::uninit()];
const FULL: [MaybeUninit<u8>; 2] = [MaybeUninit::new(1), MaybeUninit::new(2)];

#[kani::proof]
fn assign_uninit_constant_ub() {
    let mut x = MaybeUninit::<u32>::new(5);
    x = UNINIT;
    let _ = unsafe { x.assume_init() };
}

#[kani::proof]
fn assign_partly_init_constant_ok() {
    let mut a = [MaybeUninit::<u8>::uninit(); 2];
    a = HALF;
    assert_eq!(unsafe { a[0].assume_init() }, 1);
}

#[inline(never)]
fn second(x: [MaybeUninit<u8>; 2]) -> u8 {
    unsafe { x[1].assume_init() }
}

#[kani::proof]
fn pass_init_constant_ok() {
    assert_eq!(second(FULL), 2);
}

/// Passing a constant with uninitialized union bytes is reported as unsupported.
#[kani::proof]
fn pass_partly_init_constant_unsupported() {
    let _ = second(HALF);
}
