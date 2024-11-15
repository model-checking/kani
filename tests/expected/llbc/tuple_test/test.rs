// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles simple tuple

fn tuple_add (t: (i32, i32)) -> i32 {
    t.0 + t.1
}

#[kani::proof]
fn main() {
    let s = tuple_add((1, 2));
}
