// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Ensure that "extern crate kani" is sufficient to run Kani when no_std is enabled.

#![no_std]
extern crate kani;

fn add(x: u8, y: u8) -> u8 {
    x + y
}

#[kani::proof]
fn prove_add() {
    let x = kani::any_where(|n| *n < u8::MAX / 2);
    let y = kani::any_where(|n| *n < u8::MAX / 2);
    add(x, y);
}

use kani::cover;

#[kani::proof]
fn verify_point() {
    cover!(true)
}
