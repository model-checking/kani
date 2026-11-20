// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles shifts, whose shift-amount check folds into
//! the operator.

fn shl(a: u32, b: u32) -> u32 {
    a << b
}
fn shr(a: i64, b: u32) -> i64 {
    a >> b
}
#[kani::proof]
fn main() {
    let _ = shl(1, 3);
    let _ = shr(-8, 1);
}
