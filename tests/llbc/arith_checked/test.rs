// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles overflow-checked arithmetic: MIR's
//! `CheckedBinaryOp` plus its overflow `Assert` fold into a panicking operator.

fn add(a: u8, b: u8) -> u8 {
    a + b
}
fn sub(a: i16, b: i16) -> i16 {
    a - b
}
fn mul(a: u32, b: u32) -> u32 {
    a * b
}
#[kani::proof]
fn main() {
    let _ = add(1, 2);
    let _ = sub(5, 2);
    let _ = mul(3, 4);
}
