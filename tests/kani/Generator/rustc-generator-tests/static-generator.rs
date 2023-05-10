// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/generator/static-generator.rs
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
    let mut generator = static || {
        let a = true;
        let b = &a;
        yield;
        assert_eq!(b as *const _, &a as *const _);
    };
    // SAFETY: We shadow the original generator variable so have no safe API to
    // move it after this point.
    let mut generator = unsafe { Pin::new_unchecked(&mut generator) };
    assert_eq!(generator.as_mut().resume(()), GeneratorState::Yielded(()));
    assert_eq!(generator.as_mut().resume(()), GeneratorState::Complete(()));
}
