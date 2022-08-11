// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/generator/nested-generators.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

#[kani::proof]
fn main() {
    let _generator = || {
        let mut sub_generator = || {
            yield 2;
        };

        match Pin::new(&mut sub_generator).resume(()) {
            GeneratorState::Yielded(x) => {
                yield x;
            }
            _ => panic!(),
        };
    };
}
