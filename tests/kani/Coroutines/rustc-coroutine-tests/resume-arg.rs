// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/resume-arg-size.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

#![feature(coroutines, coroutine_trait)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

// run-pass

use std::mem::size_of_val;

#[kani::proof]
fn main() {
    // Coroutine taking a `Copy`able resume arg.
    let mut gen_copy = |mut x: usize| {
        loop {
            drop(x);
            x = yield;
        }
    };

    // Coroutine taking a non-`Copy` resume arg.
    let mut gen_move = |mut x: Box<usize>| {
        loop {
            drop(x);
            x = yield;
        }
    };

    assert_eq!(Pin::new(&mut gen_copy).resume(0), CoroutineState::Yielded(()));
    assert_eq!(Pin::new(&mut gen_copy).resume(1), CoroutineState::Yielded(()));

    assert_eq!(Pin::new(&mut gen_move).resume(Box::new(0)), CoroutineState::Yielded(()));
    assert_eq!(Pin::new(&mut gen_move).resume(Box::new(1)), CoroutineState::Yielded(()));
}
