// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing
//! Check that Kani resolves a qualified inherent-method stub target (`<Foo>::foo`) on a
//! struct with const generics. The plain `Foo::foo` spelling is covered by the sibling
//! `stubbing-const-generics` test; this covers the qualified spelling, which routes
//! through the type resolver.

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
#[kani::stub(<Foo>::foo, bar)]
fn main() {
    let f = Foo { _a: [1, 2, 3], x: true };
    assert!(!f.foo());
}
