// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `needs_drop` intrinsic

use std::mem;

pub struct Foo<T> {
    _foo: T,
}

impl<T> Foo<T> {
    fn call_needs_drop(&self) -> bool {
        return const { mem::needs_drop::<T>() };
    }
}

#[kani::proof]
fn main() {
    // Integers don't need to be dropped
    let int_foo = Foo::<i32> { _foo: 0 };
    assert!(!int_foo.call_needs_drop());

    // But strings do need to be dropped
    let string_foo = Foo::<String> { _foo: "".to_string() };
    assert!(string_foo.call_needs_drop());
}
