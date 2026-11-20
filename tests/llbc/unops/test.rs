// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles unary negation and bitwise/logical not.

fn neg(a: i32) -> i32 {
    -a
}
fn not(a: u8) -> u8 {
    !a
}
fn lnot(a: bool) -> bool {
    !a
}
#[kani::proof]
fn main() {
    let _ = neg(3);
    let _ = not(1);
    let _ = lnot(true);
}
