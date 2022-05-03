// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that we can handle potential naming conflicts when
// generic types give the same Trait::function name pairs, even when
// non-object-safe methods force the vtable to have VtblEntry::Vacant
// positions

trait Foo<T> {
    // Non-object-safe method first, so the vtable has
    // a vacant spot before the important method
    fn new() -> Self
    where
        Self: Sized;

    fn method(&self, t: T) -> T;
}

trait Bar: Foo<u32> + Foo<i32> {}

impl<T> Foo<T> for () {
    fn new() -> Self {
        unimplemented!()
    }

    fn method(&self, t: T) -> T {
        t
    }
}

impl Bar for () {}

#[kani::proof]
fn main() {
    let b: &dyn Bar = &();
    // The vtable for b will now have two Foo::method entries,
    // one for Foo<u32> and one for Foo<i32>. Both follow the
    // vacant vtable entries for Foo<u32>::new and Foo<i32>::new
    // which are not object safe.
    let result = <dyn Bar as Foo<u32>>::method(b, 22_u32);
    assert!(result == 22_u32);
}
