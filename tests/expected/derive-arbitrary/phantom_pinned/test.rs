// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Arbitrary` on a struct that has
//! `std::marker::PhantomPinned`

#[derive(kani::Arbitrary)]
struct Foo {
    x: i32,
    _f: std::marker::PhantomPinned,
}

impl Foo {
    fn new(v: i32) -> Self {
        Self { x: v, _f: std::marker::PhantomPinned }
    }
}

#[kani::proof]
fn check_arbitrary_phantom_pinned() {
    let x = kani::any();
    let f: Foo = Foo::new(x);
    assert_eq!(f.x, x);
}
