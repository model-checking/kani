#![feature(never_type)]

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0006]
                            //~| [proptest_derive, E0008]
enum NonFatal<#[proptest(skip)] T> {
    #[proptest(skip)]
    Unit(T),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T0 {
    #[proptest(skip)]
    Unit,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T1 {
    #[proptest(skip)]
    V0,
    #[proptest(skip)]
    V1,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T2 {
    #[proptest(skip)]
    V0,
    #[proptest(skip)]
    V1,
    V2(!),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0006]
enum T3 {
    #[proptest(skip)]
    V0,
    #[proptest(skip)]
    V1,
    V2([!; 1 + 2 + (3 / 3) + (1 << 3)]),
}
