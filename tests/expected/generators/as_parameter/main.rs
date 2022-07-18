// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that includes a generator as a parameter to a function
// from https://github.com/model-checking/kani/issues/1075

#![feature(generators, generator_trait)]

use std::ops::Generator;

fn foo<T>(g: T)
where
    T: Generator,
{
    let _ = g;
}

#[kani::proof]
fn main() {
    foo(|| {
        yield 1;
        return 2;
    });
}
