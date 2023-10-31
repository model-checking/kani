// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/smoke.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass

// revisions: default nomiropt
//[nomiropt]compile-flags: -Z mir-opt-level=0

// ignore-emscripten no threads support
// compile-flags: --test

#![feature(coroutines, coroutine_trait)]

use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;
use std::thread;

#[kani::proof]
fn simple() {
    let mut foo = || {
        if false {
            yield;
        }
    };

    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Complete(()) => {}
        s => panic!("bad state: {:?}", s),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn return_capture() {
    let a = String::from("foo");
    let mut foo = || {
        if false {
            yield;
        }
        a
    };

    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Complete(ref s) if *s == "foo" => {}
        s => panic!("bad state: {:?}", s),
    }
}

#[kani::proof]
fn simple_yield() {
    let mut foo = || {
        yield;
    };

    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Yielded(()) => {}
        s => panic!("bad state: {:?}", s),
    }
    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Complete(()) => {}
        s => panic!("bad state: {:?}", s),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn yield_capture() {
    let b = String::from("foo");
    let mut foo = || {
        yield b;
    };

    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Yielded(ref s) if *s == "foo" => {}
        s => panic!("bad state: {:?}", s),
    }
    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Complete(()) => {}
        s => panic!("bad state: {:?}", s),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn simple_yield_value() {
    let mut foo = || {
        yield String::from("bar");
        return String::from("foo");
    };

    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Yielded(ref s) if *s == "bar" => {}
        s => panic!("bad state: {:?}", s),
    }
    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Complete(ref s) if *s == "foo" => {}
        s => panic!("bad state: {:?}", s),
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn return_after_yield() {
    let a = String::from("foo");
    let mut foo = || {
        yield;
        return a;
    };

    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Yielded(()) => {}
        s => panic!("bad state: {:?}", s),
    }
    match Pin::new(&mut foo).resume(()) {
        CoroutineState::Complete(ref s) if *s == "foo" => {}
        s => panic!("bad state: {:?}", s),
    }
}

// This test is useless for Kani
fn send_and_sync() {
    assert_send_sync(|| yield);
    assert_send_sync(|| {
        yield String::from("foo");
    });
    assert_send_sync(|| {
        yield;
        return String::from("foo");
    });
    let a = 3;
    assert_send_sync(|| {
        yield a;
        return;
    });
    let a = 3;
    assert_send_sync(move || {
        yield a;
        return;
    });
    let a = String::from("a");
    assert_send_sync(|| {
        yield;
        drop(a);
        return;
    });
    let a = String::from("a");
    assert_send_sync(move || {
        yield;
        drop(a);
        return;
    });

    fn assert_send_sync<T: Send + Sync>(_: T) {}
}

// Kani does not support threads, so we cannot run this test:
fn send_over_threads() {
    let mut foo = || yield;
    thread::spawn(move || {
        match Pin::new(&mut foo).resume(()) {
            CoroutineState::Yielded(()) => {}
            s => panic!("bad state: {:?}", s),
        }
        match Pin::new(&mut foo).resume(()) {
            CoroutineState::Complete(()) => {}
            s => panic!("bad state: {:?}", s),
        }
    })
    .join()
    .unwrap();

    let a = String::from("a");
    let mut foo = || yield a;
    thread::spawn(move || {
        match Pin::new(&mut foo).resume(()) {
            CoroutineState::Yielded(ref s) if *s == "a" => {}
            s => panic!("bad state: {:?}", s),
        }
        match Pin::new(&mut foo).resume(()) {
            CoroutineState::Complete(()) => {}
            s => panic!("bad state: {:?}", s),
        }
    })
    .join()
    .unwrap();
}
