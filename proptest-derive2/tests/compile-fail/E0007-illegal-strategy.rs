// Copyright 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0007]
                            //~| [proptest_derive, E0030]
#[proptest(params = "u8")]
#[proptest(strategy = "1u8..")]
struct A {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct B;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct C();

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct D { field: String }

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct E(Vec<u8>);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
enum F { V1, V2, }
