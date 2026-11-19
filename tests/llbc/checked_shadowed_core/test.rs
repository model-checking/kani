// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! A crate that defines its own `mod core`. The LLBC backend looks up the `*_with_overflow`
//! intrinsic for a checked operation by path; that lookup must resolve `core` as the external
//! crate, not as this module.

#[allow(dead_code)]
mod core {}

fn add(a: u8, b: u8) -> u8 {
    a + b
}

#[kani::proof]
fn main() {
    let _ = add(1, 2);
}
