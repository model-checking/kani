// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing
//! Check that Kani supports stubbing methods for structs with const generics.
//! This was previously crashing:
//! https://github.com/model-checking/kani/issues/4322

struct Foo<const C: usize> {
    _a: [i32; C],
    x: bool,
}

impl<const C: usize> Foo<C> {
    fn foo(&self) -> bool {
        self.x
    }
}

pub fn bar<const C: usize>(x: &Foo<C>) -> bool {
    false
}

#[kani::proof]
#[kani::stub(Foo::foo, bar)]
fn main() {
    let f = Foo { _a: [1, 2, 3], x: true };
    assert!(!f.foo());
}
