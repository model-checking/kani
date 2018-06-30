#![feature(attr_literals)]

#[macro_use]
extern crate proptest_derive;
extern crate proptest;

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
