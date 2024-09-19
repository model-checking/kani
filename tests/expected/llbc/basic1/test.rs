// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zaeneas

//! This test checks that Kani's LLBC backend handles basic expressions, in this
//! case an if condition

fn select(s: bool, x: i32, y: i32) -> i32 {
    if s {
        x
    } else {
        y
    }
}


#[kani::proof]
fn main() {
    let _ = select(true, 3, 7);
}
