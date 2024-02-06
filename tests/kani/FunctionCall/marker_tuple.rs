// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test that Kani can properly handle the "rust-call" ABI with an empty tuple.
//! Issue first reported here: <https://github.com/model-checking/kani/issues/2260>

#![feature(unboxed_closures, tuple_trait)]

extern "rust-call" fn foo<T: std::marker::Tuple>(_: T) {}

#[kani::proof]
fn main() {
    foo(());
}
