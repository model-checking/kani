// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test contains a call to a coroutine via a Pin
// from https://github.com/model-checking/kani/issues/416

#![feature(coroutines, coroutine_trait)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

#[kani::proof]
fn main() {
    let mut coroutine = || {
        yield 1;
        return true;
    };

    match Pin::new(&mut coroutine).resume(()) {
        CoroutineState::Yielded(1) => {}
        _ => panic!("unexpected return from resume"),
    }
    match Pin::new(&mut coroutine).resume(()) {
        CoroutineState::Complete(true) => {}
        _ => panic!("unexpected yield from resume"),
    }
}
