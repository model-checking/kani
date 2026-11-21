// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// compile-flags: --edition 2021
// Regression for #5072: volatile ZST reads must not access memory.
#![feature(core_intrinsics)]

use std::ptr::{self, NonNull};

#[derive(Debug, PartialEq)]
#[repr(align(8))]
struct Z;

#[kani::proof]
fn read_volatile_unit_reference() {
    assert_eq!(unsafe { ptr::read_volatile(&()) }, ());
}

#[kani::proof]
fn read_volatile_dangling_unit() {
    let src = NonNull::<()>::dangling().as_ptr();
    assert_eq!(unsafe { ptr::read_volatile(src) }, ());
}

#[kani::proof]
fn read_volatile_overaligned_zst() {
    let src = NonNull::<Z>::dangling().as_ptr();
    assert_eq!(unsafe { ptr::read_volatile(src) }, Z);
}

#[kani::proof]
fn read_volatile_null_zsts() {
    // The ZST exception permits null; zero is aligned for both types.
    assert_eq!(unsafe { ptr::read_volatile(ptr::null::<()>()) }, ());
    assert_eq!(unsafe { ptr::read_volatile(ptr::null::<Z>()) }, Z);
}

#[kani::proof]
fn intrinsic_volatile_load_zsts() {
    // Exercise lowering directly as well as through the public API.
    let src = NonNull::<Z>::dangling().as_ptr();
    assert_eq!(unsafe { std::intrinsics::volatile_load(src) }, Z);
    assert_eq!(unsafe { std::intrinsics::volatile_load(ptr::null::<()>()) }, ());
}

#[kani::proof]
fn read_volatile_aggregate_zsts() {
    // Zero-sized arrays and nested fields also need a correctly typed result.
    let src = NonNull::<([Z; 2], [u8; 0])>::dangling().as_ptr();
    let (zs, bytes) = unsafe { ptr::read_volatile(src) };
    assert_eq!(zs, [Z, Z]);
    assert_eq!(bytes, []);
}
