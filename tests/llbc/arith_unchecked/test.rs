// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles unchecked arithmetic (`unchecked_*`
//! intrinsics), whose overflow is undefined behavior.

#![feature(core_intrinsics)]
#![allow(internal_features)]

use std::intrinsics::{unchecked_add, unchecked_mul, unchecked_sub};
fn add(a: u8, b: u8) -> u8 {
    unsafe { unchecked_add(a, b) }
}
fn sub(a: i16, b: i16) -> i16 {
    unsafe { unchecked_sub(a, b) }
}
fn mul(a: u32, b: u32) -> u32 {
    unsafe { unchecked_mul(a, b) }
}
#[kani::proof]
fn main() {
    let _ = add(1, 2);
    let _ = sub(5, 2);
    let _ = mul(3, 4);
}
