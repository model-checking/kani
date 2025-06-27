// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/static-coroutine.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass

#![feature(coroutines, coroutine_trait)]
#![feature(stmt_expr_attributes)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

#[kani::proof]
fn main() {
    let mut coroutine = #[coroutine]
    static || {
        let a = true;
        let b = &a;
        yield;
        assert_eq!(b as *const _, &a as *const _);
    };
    // SAFETY: We shadow the original coroutine variable so have no safe API to
    // move it after this point.
    let mut coroutine = unsafe { Pin::new_unchecked(&mut coroutine) };
    assert_eq!(coroutine.as_mut().resume(()), CoroutineState::Yielded(()));
    assert_eq!(coroutine.as_mut().resume(()), CoroutineState::Complete(()));
}
