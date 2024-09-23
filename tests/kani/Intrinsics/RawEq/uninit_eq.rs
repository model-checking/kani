// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Checks that `raw_eq` cannot determine equality when one of the arguments is
// uninitialized
#![feature(core_intrinsics)]
use std::intrinsics::raw_eq;
use std::mem::{MaybeUninit, zeroed};

#[kani::proof]
fn main() {
    let zeroed_arr: [u8; 8] = unsafe { zeroed() };
    let uninit_arr: [u8; 8] = unsafe { MaybeUninit::uninit().assume_init() };

    let arr_are_eq = unsafe { raw_eq(&zeroed_arr, &uninit_arr) };
    assert!(arr_are_eq);
}
