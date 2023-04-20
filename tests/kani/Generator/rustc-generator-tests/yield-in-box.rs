// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/generator/yield-in-box.rs

// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass
// Test that box-statements with yields in them work.

#![feature(generators, generator_trait)]
use std::ops::Generator;
use std::ops::GeneratorState;
use std::pin::Pin;

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let x = 0i32;
    || {
        //~ WARN unused generator that must be used
        let y = 2u32;
        {
            let _t = Box::new((&x, yield 0, &y));
        }
        match Box::new((&x, yield 0, &y)) {
            _t => {}
        }
    };

    let mut g = |_| Box::new(yield);
    assert_eq!(Pin::new(&mut g).resume(1), GeneratorState::Yielded(()));
    assert_eq!(Pin::new(&mut g).resume(2), GeneratorState::Complete(Box::new(2)));
}
