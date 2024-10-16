// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! https://github.com/rust-lang/rust/blob/7c4ac0603e9ee5295bc802c90575391288a69a8a/library/core/src/slice/memchr.rs#L38C10-L38C22

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

const fn memchr_naive(x: u8, text: &[u8]) -> Option<usize> {
    let mut i = 0;

    // #[kani::loop_invariant(i <= text.len() && ((i == 0) || text[i-1]!= x))]
    while i < text.len() {
        if text[i] == x {
            return Some(i);
        }

        i += 1;
    }

    None
}

#[kani::proof]
fn main() {
    let mut text = [1; 20];
    text[4] = 5;
    let x = 5;

    assert_eq!(memchr_naive(x, &text), Some(4));
}
