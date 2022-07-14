// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can codegen code that has a Generator type present,
// when the path is not dynamically used.
#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};

// Separate function to force translation
fn maybe_call(call: bool) {
    if call {
        let mut _generator = || {
            yield 1;
            return 2;
        };
    } else {
        assert!(1 + 1 == 2);
    }
}

#[kani::proof]
fn main() {
    maybe_call(false);
}
