// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/generator/size-moved-locals.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// run-pass
// Test that we don't duplicate storage for a variable that is moved to another
// binding. This used to happen in the presence of unwind and drop edges (see
// `complex` below.)
//
// The exact sizes here can change (we'd like to know when they do). What we
// don't want to see is the `complex` generator size being upwards of 2048 bytes
// (which would indicate it is reserving space for two copies of Foo.)
//
// See issue #59123 for a full explanation.

// edition:2018
// ignore-wasm32 issue #62807
// ignore-asmjs issue #62807

#![feature(generators, generator_trait)]

use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

const FOO_SIZE: usize = 128;
struct Foo([u8; FOO_SIZE]);

impl Drop for Foo {
    fn drop(&mut self) {}
}

fn move_before_yield() -> impl Generator<Yield = (), Return = ()> {
    static || {
        let first = Foo([0; FOO_SIZE]);
        let _second = first;
        yield;
        // _second dropped here
    }
}

fn noop() {}

fn move_before_yield_with_noop() -> impl Generator<Yield = (), Return = ()> {
    static || {
        let first = Foo([0; FOO_SIZE]);
        noop();
        let _second = first;
        yield;
        // _second dropped here
    }
}

// Today we don't have NRVO (we allocate space for both `first` and `second`,)
// but we can overlap `first` with `_third`.
fn overlap_move_points() -> impl Generator<Yield = (), Return = ()> {
    static || {
        let first = Foo([0; FOO_SIZE]);
        yield;
        let second = first;
        yield;
        let _third = second;
        yield;
    }
}

fn overlap_x_and_y() -> impl Generator<Yield = (), Return = ()> {
    static || {
        let x = Foo([0; FOO_SIZE]);
        yield;
        drop(x);
        let y = Foo([0; FOO_SIZE]);
        yield;
        drop(y);
    }
}

#[kani::proof]
fn main() {
    let mut generator = move_before_yield();
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Yielded(())
    );
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Complete(())
    );

    let mut generator = move_before_yield_with_noop();
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Yielded(())
    );
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Complete(())
    );

    let mut generator = overlap_move_points();
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Yielded(())
    );
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Yielded(())
    );
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Yielded(())
    );
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Complete(())
    );

    let mut generator = overlap_x_and_y();
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Yielded(())
    );
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Yielded(())
    );
    assert_eq!(
        unsafe { Pin::new_unchecked(&mut generator) }.resume(()),
        GeneratorState::Complete(())
    );
}
