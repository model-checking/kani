// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A `become` in a function the harness never calls does not affect verification.
//! <https://github.com/model-checking/kani/issues/5131>

#![feature(explicit_tail_calls)]
#![allow(incomplete_features)]

#[allow(dead_code)]
fn g(x: u32) -> u32 {
    x + 1
}

#[allow(dead_code)]
fn f(x: u32) -> u32 {
    become g(x)
}

#[kani::proof]
fn unreachable_tail_call() {
    assert_eq!(1 + 1, 2);
}
