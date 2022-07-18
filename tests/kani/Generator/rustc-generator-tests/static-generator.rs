// Copyright rustc Contributors
// SPDX-License-Identifier: Apache OR MIT
// Adapted from rustc: src/test/ui/generator/static-generator.rs
// Changes: copyright Kani contributors, Apache or MIT

// run-pass

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

#[kani::proof]
#[kani::unwind(2)]
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
