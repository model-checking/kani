// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --zero-init-vars

// Checks variables are zero_initialized when the flag is set
#![feature(core_intrinsics)]
use std::intrinsics::raw_eq;
use std::mem::{zeroed, MaybeUninit};

#[kani::proof]
fn main() {
    let zeroed_arr: [u8; 8] = unsafe { zeroed() };
    let uninit_arr: [u8; 8] = unsafe { MaybeUninit::uninit().assume_init() };

    let arr_are_eq = unsafe { raw_eq(&zeroed_arr, &uninit_arr) };
    assert!(arr_are_eq);
}
