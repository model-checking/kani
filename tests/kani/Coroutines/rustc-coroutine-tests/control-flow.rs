// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/control-flow.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass

// revisions: default nomiropt
//[nomiropt]compile-flags: -Z mir-opt-level=0

#![feature(coroutines, coroutine_trait)]

use std::marker::Unpin;
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

fn finish<T>(mut amt: usize, mut t: T) -> T::Return
where
    T: Coroutine<(), Yield = ()> + Unpin,
{
    loop {
        match Pin::new(&mut t).resume(()) {
            CoroutineState::Yielded(()) => amt = amt.checked_sub(1).unwrap(),
            CoroutineState::Complete(ret) => {
                assert_eq!(amt, 0);
                return ret;
            }
        }
    }
}

#[kani::proof]
#[kani::unwind(16)]
fn main() {
    finish(
        1,
        #[coroutine]
        || yield,
    );
    finish(
        8,
        #[coroutine]
        || {
            for _ in 0..8 {
                yield;
            }
        },
    );
    finish(
        1,
        #[coroutine]
        || {
            if true {
                yield;
            } else {
            }
        },
    );
    finish(
        1,
        #[coroutine]
        || {
            if false {
            } else {
                yield;
            }
        },
    );
    finish(
        2,
        #[coroutine]
        || {
            if {
                yield;
                false
            } {
                yield;
                panic!()
            }
            yield
        },
    );
}
