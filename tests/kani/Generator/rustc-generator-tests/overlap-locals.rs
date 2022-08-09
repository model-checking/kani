// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/generator/overlap-locals.rs
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
    let mut a = || {
        {
            let w: i32 = 4;
            yield;
        }
        {
            let x: i32 = 5;
            yield;
        }
        {
            let y: i32 = 6;
            yield;
        }
        {
            let z: i32 = 7;
            yield;
        }
    };

    assert_eq!(Pin::new(&mut a).resume(()), GeneratorState::Yielded(()));
    assert_eq!(Pin::new(&mut a).resume(()), GeneratorState::Yielded(()));
    assert_eq!(Pin::new(&mut a).resume(()), GeneratorState::Yielded(()));
    assert_eq!(Pin::new(&mut a).resume(()), GeneratorState::Yielded(()));
    assert_eq!(Pin::new(&mut a).resume(()), GeneratorState::Complete(()));
}
