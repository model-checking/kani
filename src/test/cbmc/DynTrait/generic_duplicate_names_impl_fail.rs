// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that we can handle potential naming conflicts when
// generic types give the same Trait::function name pairs. This test
// gives different concrete impls for i32 and u32. This _fail test
// expects the wrong result.

include!("../../rmc-prelude.rs");

trait Foo<T> {
    fn method(&self, t: T) -> T;
}

trait Bar: Foo<u32> + Foo<i32> {}

impl Foo<i32> for () {
    fn method(&self, t: i32) -> i32 {
        0
    }
}

impl Foo<u32> for () {
    fn method(&self, t: u32) -> u32 {
        t
    }
}

impl Bar for () {}

fn main() {
    let b: &dyn Bar = &();
    // The vtable for b will now have two Foo::method entries,
    // one for Foo<u32> and one for Foo<i32>.
    let result = <dyn Bar as Foo<u32>>::method(b, 22_u32);
    __VERIFIER_expect_fail(result == 0, "Wrong function");

    let result = <dyn Bar as Foo<i32>>::method(b, 22_i32);
    __VERIFIER_expect_fail(result == 22_i32, "Wrong function");
}
