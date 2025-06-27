// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Arbitrary` on a struct with a
//! member of type `Box<T>`

#[derive(kani::Arbitrary)]
struct Foo<T> {
    boxed: Box<T>,
}

#[kani::proof]
fn main() {
    let foo: Foo<i32> = kani::any();
    kani::cover!(*foo.boxed == i32::MIN);
    kani::cover!(*foo.boxed == 0);
    kani::cover!(*foo.boxed == i32::MAX);
    kani::cover!(*foo.boxed < i32::MIN); // <-- this condition should be `UNSATISFIABLE`
    kani::cover!(*foo.boxed > i32::MAX); // <-- this condition should be `UNSATISFIABLE`
}
