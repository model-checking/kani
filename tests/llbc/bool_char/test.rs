// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles `bool` and `char` literals.

fn is_a(c: char) -> bool {
    c == 'a'
}
fn always() -> bool {
    true
}
#[kani::proof]
fn main() {
    let _ = is_a('b');
    let _ = always();
}
