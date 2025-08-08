// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Ensure that a no_std crate with no harnesses & no "extern crate kani" gets a useful error message.

#![no_std]

fn add(x: u8, y: u8) -> u8 {
    x + y
}
