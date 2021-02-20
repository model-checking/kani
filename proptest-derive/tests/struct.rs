// Copyright 2019 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use proptest::prelude::Arbitrary;
use proptest_derive::Arbitrary;

#[derive(Debug, Arbitrary)]
struct T1 {
    f1: u8
}

#[derive(Debug, Arbitrary)]
struct T10 {
    f1: char,
    f2: String,
    f3: u8,
    f4: u16,
    f5: u32,
    f6: u64,
    f7: u128,
    f8: f32,
    f9: f64,
    f10: bool,
}

#[derive(Debug, Arbitrary)]
struct T11 {
    f1: char,
    f2: String,
    f3: u8,
    f4: u16,
    f5: u32,
    f6: u64,
    f7: u128,
    f8: f32,
    f9: f64,
    f10: bool,
    f11: char,
}

#[derive(Debug, Arbitrary)]
struct T13 {
    f1: char,
    f2: String,
    f3: u8,
    f4: u16,
    f5: u32,
    f6: u64,
    f7: u128,
    f8: f32,
    f9: f64,
    f10: bool,
    f11: char,
    f12: String,
    f13: u8,
}

#[derive(Debug, Arbitrary)]
struct T19 {
    f1: char,
    f2: String,
    f3: u8,
    f4: u16,
    f5: u32,
    f6: u64,
    f7: u128,
    f8: f32,
    f9: f64,
    f10: bool,
    f11: char,
    f12: String,
    f13: u8,
    f14: u16,
    f15: u32,
    f16: u64,
    f17: u128,
    f18: f32,
    f19: f64,
}

#[derive(Debug, Arbitrary)]
struct T20 {
    f1: char,
    f2: String,
    f3: u8,
    f4: u16,
    f5: u32,
    f6: u64,
    f7: u128,
    f8: f32,
    f9: f64,
    f10: bool,
    f11: char,
    f12: String,
    f13: u8,
    f14: u16,
    f15: u32,
    f16: u64,
    f17: u128,
    f18: f32,
    f19: f64,
    f20: bool
}

#[test]
fn asserting_arbitrary() {
    fn assert_arbitrary<T: Arbitrary>() {}

    assert_arbitrary::<T1>();
    assert_arbitrary::<T10>();
    assert_arbitrary::<T11>();
    assert_arbitrary::<T13>();
    assert_arbitrary::<T19>();
    assert_arbitrary::<T20>();
}
