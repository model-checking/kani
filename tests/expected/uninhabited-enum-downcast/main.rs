// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! An enum whose only variant holds `!` gets the layout of `!`, with no fields to project into.
//! Downcasting into that variant used to crash the compiler
//! (<https://github.com/model-checking/kani/issues/3596>). Check that it is reported as an
//! unsupported construct instead.

#![feature(never_type)]
#[derive(Copy, Clone)]
pub enum E {
    A(!),
}
pub union U {
    u: (),
    e: E,
}

#[kani::proof]
fn check_let_ref() {
    let E::A(ref _a) = unsafe { &(&U { u: () }).e };
}

#[kani::proof]
fn check_match_ref() {
    match unsafe { &(&U { u: () }).e } {
        E::A(_a) => {}
    }
}
