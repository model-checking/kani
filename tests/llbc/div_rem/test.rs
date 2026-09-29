// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles division and remainder, whose division-by-zero
//! and overflow checks fold into the operator.

fn div(a: u32, b: u32) -> u32 {
    a / b
}
fn rem(a: i32, b: i32) -> i32 {
    a % b
}
#[kani::proof]
fn main() {
    let _ = div(7, 2);
    let _ = rem(7, 2);
}
