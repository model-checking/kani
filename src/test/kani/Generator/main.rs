// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can codegen code that has a Generator type present,
// as long as the path is not dynamically used. Full Generator support
// tracked in: https://github.com/model-checking/rmc/issues/416

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};

// Seperate function to force translation
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

fn main() {
    maybe_call(false);
}
