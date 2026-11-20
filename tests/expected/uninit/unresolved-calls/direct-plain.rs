// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
// compile-flags: -D warnings

#[inline(never)]
fn direct(value: u8) -> u8 {
    value
}

#[kani::proof]
fn check() {
    assert_eq!(direct(42), 42);
}
