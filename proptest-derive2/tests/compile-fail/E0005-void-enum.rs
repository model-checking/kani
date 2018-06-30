#![feature(never_type)]

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0005]
enum T0 {
    V0(!),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0005]
enum T1 {
    V0(!, bool),
    V1([!; 1]),
    V2([(!, bool); 1])
}
