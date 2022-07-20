// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that a generator can be passed as a parameter.
// (adapted from https://github.com/model-checking/kani/issues/1075)

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

fn foo<G: Generator<Yield = u8, Return = u8> + Unpin>(mut g: G)
where
    <G as std::ops::Generator>::Return: std::cmp::PartialEq,
{
    let res = Pin::new(&mut g).resume(());
    assert_eq!(res, GeneratorState::Yielded(1));
    let res2 = Pin::new(&mut g).resume(());
    assert_eq!(res2, GeneratorState::Complete(2));
}

#[kani::proof]
fn main() {
    foo(|| {
        yield 1;
        return 2;
    });
}
