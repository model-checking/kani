// Copyright rustc Contributors
// SPDX-License-Identifier: Apache OR MIT
// Adapted from rustc: src/test/ui/generator/yield-in-box.rs
// Changes: copyright Kani contributors, Apache or MIT

// run-pass
// Test that box-statements with yields in them work.

#![feature(generators, box_syntax, generator_trait)]
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
            let _t = box (&x, yield 0, &y);
        }
        match box (&x, yield 0, &y) {
            _t => {}
        }
    };

    let mut g = |_| box yield;
    assert_eq!(Pin::new(&mut g).resume(1), GeneratorState::Yielded(()));
    assert_eq!(Pin::new(&mut g).resume(2), GeneratorState::Complete(box 2));
}
