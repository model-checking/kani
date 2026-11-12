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

#[kani::proof]
fn check() {
    assert_eq!(indirect(identity, 42), 42);
}
