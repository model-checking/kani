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

    assert_arbitrary::<T0>();
    assert_arbitrary::<T1>();
    assert_arbitrary::<T2>();
}

proptest! {
    #[test]
    fn t0_outty_42(t: T0) {
        prop_assert_eq!(t.field.val, 42);
    }

    #[test]
    fn t1_no_panic(_: T1) {}

    #[test]
    fn t2_no_panic(_: T1) {}

    #[test]
    fn t3_no_panic(_: T1) {}
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
struct T0 {
    field: <Input as Func>::Out,
}

#[derive(Debug, Arbitrary)]
struct T1 {
    field: Vec<u8>,
}

#[derive(Debug, Arbitrary)]
struct T2 {
    field: Vec<Vec<u8>>,
}

#[derive(Debug, Arbitrary)]
struct T3 {
    field: Vec<<Input as Func>::Out>,
}