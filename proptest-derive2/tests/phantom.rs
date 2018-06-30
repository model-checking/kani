// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code, unused_variables, unused_imports)]

#[macro_use]
extern crate proptest_derive;
extern crate proptest;
use proptest::prelude::Arbitrary;

use std::marker;
use std::marker::PhantomData;

#[derive(Debug)]
struct T0;

#[derive(Debug, Arbitrary)]
struct T1<T>(::std::marker::PhantomData<T>);

#[derive(Debug, Arbitrary)]
struct T2(T1<T0>);

#[derive(Debug, Arbitrary)]
struct T3<T>(marker::PhantomData<T>);

#[derive(Debug, Arbitrary)]
struct T4(T3<T0>);

#[derive(Debug, Arbitrary)]
struct T5<T>(PhantomData<T>);

#[derive(Debug, Arbitrary)]
struct T6(T5<T0>);

#[derive(Debug, Arbitrary)]
struct T7<T>(std::marker::PhantomData<T>);

#[derive(Debug, Arbitrary)]
struct T8(T7<T0>);

#[derive(Debug, Arbitrary)]
struct T9<A, B, C> {
    a: A,
    b: B,
    c: PhantomData<C>,
}

fn assert_arbitrary<T: Arbitrary>() {}

#[test]
fn asserting_t9_arbitrary() {
    assert_arbitrary::<T9<u8, usize, T0>>();
}
