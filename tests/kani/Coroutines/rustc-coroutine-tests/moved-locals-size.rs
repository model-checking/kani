// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/size-moved-locals.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! Tests the size of coroutines
//! Note that the size of coroutines can depend on the panic strategy.
//! This is the case here (see the bottom of the file).
//! In particular, running rustc with default options on this file will fail an assertion.
//! Since Kani uses "panic=abort", you need to run rustc with `-C panic=abort`
//! to get the same results as Kani.
//! More information: https://github.com/rust-lang/rust/issues/59123

// run-pass
// Test that we don't duplicate storage for a variable that is moved to another
// binding. This used to happen in the presence of unwind and drop edges (see
// `complex` below.)
//
// The exact sizes here can change (we'd like to know when they do). What we
// don't want to see is the `complex` coroutine size being upwards of 2048 bytes
// (which would indicate it is reserving space for two copies of Foo.)
//
// See issue https://github.com/rust-lang/rust/issues/59123 for a full explanation.

// edition:2018
// ignore-wasm32 issue #62807
// ignore-asmjs issue #62807

#![feature(coroutines, coroutine_trait)]

use std::ops::Coroutine;

const FOO_SIZE: usize = 1024;
struct Foo([u8; FOO_SIZE]);

impl Drop for Foo {
    fn drop(&mut self) {}
}

fn move_before_yield() -> impl Coroutine<Yield = (), Return = ()> {
    #[coroutine]
    static || {
        let first = Foo([0; FOO_SIZE]);
        let _second = first;
        yield;
        // _second dropped here
    }
}

fn noop() {}

fn move_before_yield_with_noop() -> impl Coroutine<Yield = (), Return = ()> {
    #[coroutine]
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
fn overlap_move_points() -> impl Coroutine<Yield = (), Return = ()> {
    #[coroutine]
    static || {
        let first = Foo([0; FOO_SIZE]);
        yield;
        let second = first;
        yield;
        let _third = second;
        yield;
    }
}

fn overlap_x_and_y() -> impl Coroutine<Yield = (), Return = ()> {
    #[coroutine]
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
    assert_eq!(1025, std::mem::size_of_val(&move_before_yield()));
    // With panic=unwind, the following assertion fails because the size increases to 1026.
    // More information here: https://github.com/rust-lang/rust/issues/59123
    assert_eq!(1025, std::mem::size_of_val(&move_before_yield_with_noop()));
    assert_eq!(2051, std::mem::size_of_val(&overlap_move_points()));
    assert_eq!(1026, std::mem::size_of_val(&overlap_x_and_y()));
}
