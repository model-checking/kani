// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: -D warnings

#[inline(never)]
fn indirect(f: fn(u8) -> u8, value: u8) -> u8 {
    f(value)
}

fn identity(value: u8) -> u8 {
    value
}

// The copy intrinsic initializes the destination and creates an analysis target.
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
    assert_eq!(indirect(identity, 42), 42);
}
