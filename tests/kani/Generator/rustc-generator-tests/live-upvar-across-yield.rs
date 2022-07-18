// Copyright rustc Contributors
// SPDX-License-Identifier: Apache OR MIT
// Adapted from rustc: src/test/ui/generator/live-upvar-across-yield.rs
// Changes: copyright Kani contributors, Apache or MIT

// run-pass

#![feature(generators, generator_trait)]

use std::ops::Generator;
use std::pin::Pin;

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let b = |_| 3;
    let mut a = || {
        b(yield);
    };
    Pin::new(&mut a).resume(());
}
