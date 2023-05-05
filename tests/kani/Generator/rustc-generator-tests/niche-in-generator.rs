// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/generator/niche-in-generator.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Test that niche finding works with captured generator upvars.

// run-pass

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

use std::mem::size_of_val;

fn take<T>(_: T) {}

#[kani::proof]
fn main() {
    let x = false;
    let mut gen1 = || {
        yield;
        take(x);
    };

    assert_eq!(Pin::new(&mut gen1).resume(()), GeneratorState::Yielded(()));
    assert_eq!(Pin::new(&mut gen1).resume(()), GeneratorState::Complete(()));
}
