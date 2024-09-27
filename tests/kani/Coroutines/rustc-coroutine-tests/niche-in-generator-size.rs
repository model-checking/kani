// Copyright rustc Contributors
// Adapted from rustc: https://github.com/rust-lang/rust/tree/5f98537eb7b5f42c246a52c550813c3cff336069/src/test/ui/coroutine/niche-in-coroutine.rs
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// Test that niche finding works with captured coroutine upvars.

// run-pass

#![feature(coroutines)]
#![feature(stmt_expr_attributes)]

use std::mem::size_of_val;

fn take<T>(_: T) {}

#[kani::proof]
fn main() {
    let x = false;
    let gen1 = #[coroutine]
    || {
        yield;
        take(x);
    };

    // FIXME(https://github.com/rust-lang/rust/issues/63818#issuecomment-2264915918):
    // niches in coroutines are disabled. Should be `assert_eq`.
    assert_ne!(size_of_val(&gen1), size_of_val(&Some(gen1)));
}
