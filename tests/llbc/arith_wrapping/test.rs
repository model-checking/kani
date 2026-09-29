// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles wrapping arithmetic. A plain MIR
//! `Add`/`Sub`/`Mul` wraps on overflow, so it must become Charon's `wrapping.` operators rather
//! than `checked.`, which produce a `(result, overflowed)` pair and made the LLBC type-incorrect:
//! `u8 := a checked.+ b`.

#![feature(core_intrinsics)]
#![allow(internal_features)]

use std::intrinsics::{wrapping_add, wrapping_mul, wrapping_sub};
fn add(a: u8, b: u8) -> u8 {
    wrapping_add(a, b)
}
fn sub(a: i16, b: i16) -> i16 {
    wrapping_sub(a, b)
}
fn mul(a: u32, b: u32) -> u32 {
    wrapping_mul(a, b)
}
#[kani::proof]
fn main() {
    let _ = add(200, 100);
    let _ = sub(-1, 2);
    let _ = mul(3, 4);
}
