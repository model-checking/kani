// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Ensure that a no_std crate works without an explicit "extern crate kani":
// the kani library is force-loaded by the driver (c.f.
// https://github.com/model-checking/kani/issues/3906).

#![no_std]

fn add(x: u8, y: u8) -> u8 {
    x.wrapping_add(y)
}

#[cfg(kani)]
#[kani::proof]
fn check_add() {
    let x: u8 = kani::any();
    let y: u8 = kani::any();
    assert!(add(x, y) == x.wrapping_add(y));
}
