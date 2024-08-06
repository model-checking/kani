// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/niche-in-coroutine.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Test that niche finding works with captured coroutine upvars.

// run-pass

#![feature(coroutines, coroutine_trait)]
#![feature(stmt_expr_attributes)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

use std::mem::size_of_val;

fn take<T>(_: T) {}

#[kani::proof]
fn main() {
    let x = false;
    let mut gen1 = #[coroutine]
    || {
        yield;
        take(x);
    };

    assert_eq!(Pin::new(&mut gen1).resume(()), CoroutineState::Yielded(()));
    assert_eq!(Pin::new(&mut gen1).resume(()), CoroutineState::Complete(()));
}
