// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unused_variables)]

#[macro_use]
extern crate proptest_derive;
#[macro_use]
extern crate proptest;
use proptest::prelude::Arbitrary;

#[test]
fn asserting_arbitrary() {
    fn assert_arbitrary<T: Arbitrary>() {}

    assert_arbitrary::<Foo>();
}

proptest! {
    #[test]
    fn foo_outty_42(foo: Foo) {
        prop_assert_eq!(foo.field.val, 42);
    }
}

#[derive(Debug, Arbitrary)]
struct OutTy {
    #[proptest(value = "42")]
    val: usize,
}

trait Func { type Out; }

struct Input;

impl Func for Input { type Out = OutTy; }

#[derive(Debug, Arbitrary)]
struct Foo {
    field: <Input as Func>::Out,
}