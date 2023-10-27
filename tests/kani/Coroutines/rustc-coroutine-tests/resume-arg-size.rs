// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/resume-arg-size.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

#![feature(coroutines)]

// run-pass

use std::mem::size_of_val;

#[kani::proof]
fn main() {
    // Coroutine taking a `Copy`able resume arg.
    let gen_copy = |mut x: usize| {
        loop {
            drop(x);
            x = yield;
        }
    };

    // Coroutine taking a non-`Copy` resume arg.
    let gen_move = |mut x: Box<usize>| {
        loop {
            drop(x);
            x = yield;
        }
    };

    // Neither of these coroutines have the resume arg live across the `yield`, so they should be
    // 1 Byte in size (only storing the discriminant)
    // FIXME: size of coroutines does not work reliably (https://github.com/model-checking/kani/issues/1395)
    assert_eq!(size_of_val(&gen_copy), 1);
    assert_eq!(size_of_val(&gen_move), 1);
}
