// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zaeneas --print-llbc

//! This test checks that Kani's LLBC backend handles basic expressions, in this
//! case an equality between a variable and a constant

fn is_zero(i: i32) -> bool {
    i == 0
}

#[kani::proof]
fn main() {
    let _ = is_zero(0);
}
