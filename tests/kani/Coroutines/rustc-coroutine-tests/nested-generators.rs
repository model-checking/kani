// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/nested-coroutines.rs
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
    let _coroutine = #[coroutine]
    || {
        let mut sub_coroutine = #[coroutine]
        || {
            yield 2;
        };

        match Pin::new(&mut sub_coroutine).resume(()) {
            CoroutineState::Yielded(x) => {
                yield x;
            }
            _ => panic!(),
        };
    };
}
