// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(attr_literals)]

#![allow(dead_code, unused_variables, unused_imports)]

#[macro_use]
extern crate proptest_derive;
#[macro_use]
extern crate proptest;
use proptest::prelude::any;

#[derive(Debug, Arbitrary)]
struct T0 {
    #[proptest(value = "42")]
    field: usize,
    #[proptest(value("24"))]
    bar: usize,
    #[proptest(value = "24 + 24usize")]
    baz: usize,
    #[proptest(value = 1337)]
    quux: usize,
    #[proptest(value(7331))]
    wibble: usize,
    #[proptest(value("3 * 2 + 3usize / 3"))]
    wobble: usize,
}   

#[derive(Debug, Arbitrary)]
struct T1(
    #[proptest(value = "24")]
    usize,
);

#[derive(Debug, Arbitrary)]
enum T2 {
    V0,
    #[proptest(value = "T2::V1 { field: 1337 }")]
    V1 {
        field: usize,
    },
}

#[derive(Debug, Arbitrary)]
enum T3 {
    V0,
    #[proptest(value = "T3::V1(7331)")]
    V1(usize),
}

#[derive(Debug, Arbitrary)]
enum T4 {
    V0,
    V1 {
        #[proptest(value = "6")]
        field: usize,
    },
}

#[derive(Debug, Arbitrary)]
enum T5 {
    V0,
    V1(
        #[proptest(value = "9")]
        usize
    ),
}

#[derive(Debug, Arbitrary)]
struct T6 {
    #[proptest(value = "\"alpha\".to_string()")]
    alpha: String,
    #[proptest(strategy = "0..100usize")]
    beta: usize,
}

proptest! {
    #[test]
    fn t0_fixed_fields(v in any::<T0>()) {
        prop_assert_eq!(v.field, 42);
        prop_assert_eq!(v.bar, 24);
        prop_assert_eq!(v.baz, 48);
        prop_assert_eq!(v.quux, 1337);
        prop_assert_eq!(v.wibble, 7331);
        prop_assert_eq!(v.wobble, 7);
    }

    #[test]
    fn t1_field_always_24(v in any::<T1>()) {
        prop_assert_eq!(v.0, 24);
    }

    #[test]
    fn t2_v1_always_1337(v in any::<T2>()) {
        if let T2::V1 { field } = v {
            prop_assert_eq!(field, 1337);
        }
    }

    #[test]
    fn t3_v1_always_7331(v in any::<T3>()) {
        if let T3::V1(v) = v {
            prop_assert_eq!(v, 7331);
        }
    }

    #[test]
    fn t4_v1_always_1337(v in any::<T4>()) {
        if let T4::V1 { field } = v {
            prop_assert_eq!(field, 6);
        }
    }

    #[test]
    fn t5_v1_always_7331(v in any::<T5>()) {
        if let T5::V1(v) = v {
            prop_assert_eq!(v, 9);
        }
    }

    #[test]
    fn t6_alpha_beta(v in any::<T6>()) {
        prop_assert_eq!(v.alpha, "alpha".to_string());
        prop_assert!(v.beta < 100);
    }
}
