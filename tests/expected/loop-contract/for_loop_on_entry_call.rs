// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check a `for` loop invariant whose `on_entry` expression calls a function.
//! The call needs a basic block of its own between the first pattern assignment and the loop
//! head, which used to make Kani panic with `not a loop head`.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn on_entry_call() {
    let a: [u8; 5] = kani::any();
    let v: Vec<u8> = vec![1, 2];
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 255 && on_entry(v.len()) == 2)]
    for x in a {
        s += x as u32;
    }
    assert!(s <= 5 * 255);
    assert!(v.len() == 2);
}
