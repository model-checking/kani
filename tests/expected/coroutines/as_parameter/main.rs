// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that a coroutine can be passed as a parameter.
// (adapted from https://github.com/model-checking/kani/issues/1075)

#![feature(coroutines, coroutine_trait)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

fn foo<G: Coroutine<Yield = u8, Return = u8> + Unpin>(mut g: G)
where
    <G as std::ops::Coroutine>::Return: std::cmp::PartialEq,
{
    let res = Pin::new(&mut g).resume(());
    assert_eq!(res, CoroutineState::Yielded(1));
    let res2 = Pin::new(&mut g).resume(());
    assert_eq!(res2, CoroutineState::Complete(2));
}

#[kani::proof]
fn main() {
    foo(|| {
        yield 1;
        return 2;
    });
}
