// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/resume-live-across-yield.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass

#![feature(coroutines, coroutine_trait)]
#![feature(stmt_expr_attributes)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

static DROP: AtomicUsize = AtomicUsize::new(0);

#[derive(PartialEq, Eq, Debug)]
struct Dropper(String);

impl Drop for Dropper {
    fn drop(&mut self) {
        DROP.fetch_add(1, Ordering::SeqCst);
    }
}

#[kani::proof]
#[kani::unwind(16)]
fn main() {
    let mut g = #[coroutine]
    |mut _d| {
        _d = yield;
        _d
    };

    let mut g = Pin::new(&mut g);

    assert_eq!(
        g.as_mut().resume(Dropper(String::from("Hello world!"))),
        CoroutineState::Yielded(())
    );
    assert_eq!(DROP.load(Ordering::Acquire), 0);
    match g.as_mut().resume(Dropper(String::from("Number Two"))) {
        CoroutineState::Complete(dropper) => {
            assert_eq!(DROP.load(Ordering::Acquire), 1);
            assert_eq!(dropper.0, "Number Two");
            drop(dropper);
            assert_eq!(DROP.load(Ordering::Acquire), 2);
        }
        _ => unreachable!(),
    }

    drop(g);
    assert_eq!(DROP.load(Ordering::Acquire), 2);
}
