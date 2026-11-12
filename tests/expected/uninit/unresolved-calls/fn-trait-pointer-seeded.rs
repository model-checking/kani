// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: -D warnings

#![feature(fn_traits)]

fn identity(value: u8) -> u8 {
    value
}

#[inline(never)]
fn indirect(f: fn(u8) -> u8) -> u8 {
    // Fn-trait invocation of a function pointer reaches the FnPtr boundary.
    // This is not coverage of a bodyless or direct-shadow Shim guard.
    std::ops::Fn::call(&f, (42,))
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
    assert_eq!(indirect(identity), 42);
}
