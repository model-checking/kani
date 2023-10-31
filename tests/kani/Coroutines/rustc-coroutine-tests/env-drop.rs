// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/env-drop.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass

// revisions: default nomiropt
//[nomiropt]compile-flags: -Z mir-opt-level=0

#![feature(coroutines, coroutine_trait)]

use std::ops::Coroutine;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

static A: AtomicUsize = AtomicUsize::new(0);

struct B;

impl Drop for B {
    fn drop(&mut self) {
        A.fetch_add(1, Ordering::SeqCst);
    }
}

#[kani::proof]
fn main() {
    t1();
    t2();
    t3();
}

fn t1() {
    let b = B;
    let mut foo = || {
        yield;
        drop(b);
    };

    let n = A.load(Ordering::SeqCst);
    drop(Pin::new(&mut foo).resume(()));
    assert_eq!(A.load(Ordering::SeqCst), n);
    drop(foo);
    assert_eq!(A.load(Ordering::SeqCst), n + 1);
}

fn t2() {
    let b = B;
    let mut foo = || {
        yield b;
    };

    let n = A.load(Ordering::SeqCst);
    drop(Pin::new(&mut foo).resume(()));
    assert_eq!(A.load(Ordering::SeqCst), n + 1);
    drop(foo);
    assert_eq!(A.load(Ordering::SeqCst), n + 1);
}

fn t3() {
    let b = B;
    let foo = || {
        yield;
        drop(b);
    };

    let n = A.load(Ordering::SeqCst);
    assert_eq!(A.load(Ordering::SeqCst), n);
    drop(foo);
    assert_eq!(A.load(Ordering::SeqCst), n + 1);
}
