// Copyright rustc Contributors
// SPDX-License-Identifier: Apache OR MIT
// Adapted from rustc: src/test/ui/generator/niche-in-generator.rs
// Changes: copyright Kani contributors, Apache or MIT

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

    assert_eq!(size_of_val(&gen1), size_of_val(&Some(gen1)));
}
