// Copyright 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
#[proptest(value = "T0(0)", strategy = "(0..6).prop_map(T1)")]
struct T0(u8);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
struct T1 {
    #[proptest(value = "1", strategy = "(0..1).prop_map(T1)")]
    field: u8
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
struct T2(
    #[proptest(value = "1", strategy = "(0..1).prop_map(T1)")]
    u8
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
enum T3 {
    V0 {
        #[proptest(value = "1", strategy = "0..1")]
        field: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
enum T4 {
    V0(
        #[proptest(value = "1", strategy = "0..1")]
        u8
    ),
}
