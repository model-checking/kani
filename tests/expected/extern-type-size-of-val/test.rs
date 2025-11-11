// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z c-ffi
//! This test checks that Kani properly handles extern types when used with size_of_val.
//! The test should panic with the message "cannot compute `size_of_val` for extern types"
//! rather than hitting an index out of bounds panic in the compiler.

#![feature(extern_types)]

use std::mem::size_of_val;

extern "C" {
    type A;
}

#[kani::proof]
#[kani::should_panic]
fn check_size_of_val_extern_type() {
    let x: &A = unsafe { &*(1usize as *const A) };
    let _ = size_of_val(x);
}
