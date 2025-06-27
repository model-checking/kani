// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests support for functions declared with "rust-call" ABI and an empty set of arguments.
#![feature(unboxed_closures, tuple_trait)]

extern "rust-call" fn foo<T: std::marker::Tuple>(_: T) -> usize {
    static mut COUNTER: usize = 0;
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

#[kani::proof]
fn main() {
    assert_eq!(foo(()), 1);
    assert_eq!(foo(()), 2);
}
