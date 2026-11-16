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
    // Calling this function pointer through the Fn trait reaches an indirect call.
    std::ops::Fn::call(&f, (42,))
}

#[kani::proof]
fn check() {
    assert_eq!(indirect(identity), 42);
}
