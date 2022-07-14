// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test contains a call to a generator via a Pin
// This is currently not supported, so Kani should error out

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

#[kani::proof]
fn main() {
    let mut generator = || {
        yield 1;
        return true;
    };

    match Pin::new(&mut generator).resume(()) {
        GeneratorState::Yielded(1) => {}
        _ => panic!("unexpected return from resume"),
    }
    match Pin::new(&mut generator).resume(()) {
        GeneratorState::Complete(true) => {}
        _ => panic!("unexpected yield from resume"),
    }
}
