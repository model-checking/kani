// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that autoharness defaults to parallel harness verification with terse output
//! (users can opt back into sequential verification with --output-format=regular).

pub fn f1(x: u8) -> u8 {
    x.wrapping_add(1)
}

pub fn f2(x: u16) -> u16 {
    x.wrapping_mul(2)
}

pub fn f3(x: u32) -> u32 {
    x ^ 0xdead_beef
}

pub fn f4(x: bool) -> bool {
    !x
}
