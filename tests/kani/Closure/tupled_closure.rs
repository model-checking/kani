// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! When a closure is put inside a `Fn` trait, the Rustc backend injects a shim that converts
//! between calling conventions.  This test ensures that the shim works correctly.
//! https://github.com/model-checking/kani/issues/678

fn h(x: u8, y: usize, o: Option<std::num::NonZeroUsize>) -> usize {
    x as usize + y
}

struct Foo {}

impl Foo {
    fn f(&self) -> usize {
        self.g(h)
    }
    fn g<F: Fn(u8, usize, Option<std::num::NonZeroUsize>) -> usize>(&self, ff: F) -> usize {
        ff(5, 22, None)
    }
}

#[kani::proof]
fn main() {
    let x = Foo {};
    assert!(x.f() == 27);
}
