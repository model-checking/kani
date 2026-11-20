// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit tail calls (`become`) are an incomplete unstable feature. Kani reports them as
//! unsupported instead of panicking inside `rustc_public`.
//! <https://github.com/model-checking/kani/issues/5131>

#![feature(explicit_tail_calls)]
#![allow(incomplete_features)]

fn g(x: u32) -> u32 {
    x + 1
}

fn f(x: u32) -> u32 {
    become g(x)
}

#[kani::proof]
fn tail_call() {
    assert_eq!(f(1), 2);
}
