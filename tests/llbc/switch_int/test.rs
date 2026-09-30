// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles a `match` on an integer, which becomes a
//! `SwitchInt` with a multi-value arm and a default.

fn classify(x: u8) -> u8 {
    match x {
        0 => 10,
        1 | 2 => 20,
        _ => 30,
    }
}
#[kani::proof]
fn main() {
    let _ = classify(1);
}
