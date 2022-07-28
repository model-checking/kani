// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/generator/niche-in-generator.rs

// Test that niche finding works with captured generator upvars.

// run-pass

#![feature(generators)]

use std::mem::size_of_val;

fn take<T>(_: T) {}

#[kani::proof]
fn main() {
    let x = false;
    let gen1 = || {
        yield;
        take(x);
    };

    // FIXME: size of generators does not work reliably (https://github.com/model-checking/kani/issues/1395)
    assert_eq!(size_of_val(&gen1), size_of_val(&Some(gen1)));
}
