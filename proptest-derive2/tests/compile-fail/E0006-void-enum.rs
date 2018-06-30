#![feature(never_type)]

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T2 {
    #[proptest(skip)]
    Unit,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T3 {
    #[proptest(skip)]
    V0,
    #[proptest(skip)]
    V1,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T4 {
    #[proptest(skip)]
    V0,
    #[proptest(skip)]
    V1,
    V2(!),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T5 {
    #[proptest(skip)]
    V0,
    #[proptest(skip)]
    V1,
    V2([!; 1 + 2 + (3 / 3) + (1 << 3)]),
}
