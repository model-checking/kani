// Copyright rustc Contributors
// SPDX-License-Identifier: Apache OR MIT
// Adapted from rustc: src/test/ui/generator/nested-generators.rs
// Changes: copyright Kani contributors, Apache or MIT

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
