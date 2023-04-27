// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Arbitrary` on a struct that has
//! `std::marker::PhantomData`

#[derive(kani::Arbitrary)]
struct Foo<T> {
    x: i32,
    _f: std::marker::PhantomData<T>,
}

impl<T> Foo<T> {
    fn new(v: i32) -> Self {
        Self { x: v, _f: std::marker::PhantomData }
    }
}

#[kani::proof]
fn main() {
    let x = kani::any();
    let f: Foo<u16> = Foo::new(x);
    assert_eq!(f.x, x);
}
