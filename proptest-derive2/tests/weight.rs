#![feature(attr_literals)]

#![allow(dead_code, unreachable_code)]

#[macro_use]
extern crate proptest_derive;

#[macro_use]
extern crate proptest;
use proptest::prelude::any;
use proptest::strategy::Strategy;

#[derive(Debug, Arbitrary)]
enum T1 {
    #[proptest(weight = "3")]
    V1,
    V2,
}

#[derive(Debug, Arbitrary)]
enum T2 {
    V1,
    #[proptest(weight("3"))]
    V2,
}

#[derive(Debug, Arbitrary)]
enum T3 {
    #[proptest(weight(3))]
    V1,
    V2,
}

#[derive(Debug, Arbitrary)]
enum T4 {
    V1,
    #[proptest(weight = 3)]
    V2,
}
