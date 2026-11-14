// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: -D warnings

use std::ffi::c_void;

fn write(pointer: *mut c_void) {
    unsafe { *pointer.cast::<u8>() = 7 };
}

#[inline(never)]
fn indirect(f: fn(*mut c_void), pointer: *mut c_void) {
    f(pointer);
}

// A copy intrinsic creates a delayed-UB analysis target without introducing UB.
#[inline(never)]
fn seed() {
    let source = 7u8;
    let mut destination = 0u8;
    unsafe { std::ptr::copy_nonoverlapping(&source, &mut destination, 1) };
    assert_eq!(destination, source);
}

#[kani::proof]
fn check() {
    seed();
    let mut value = 0u8;
    // c_void and u8 have the same one-byte data layout; erasure itself adds no seed.
    indirect(write, (&raw mut value).cast());
    assert_eq!(value, 7);
}
