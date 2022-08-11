// Copyright 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
#[macro_use]
extern crate proptest_derive;

fn main() {}

fn make_regex() -> &'static str {
    "a|b"
}

// struct:

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
struct T0 {
    #[proptest(regex = "a+")]
    f0: (),
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
struct T1 {
    #[proptest(regex("a*"))]
    f0: u8,
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
struct T2 {
    #[proptest(regex(make_regex))]
    f0: Vec<u16>,
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
struct T3(
    #[proptest(regex = "a+")]
    (),
);

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
struct T4(
    #[proptest(regex("a*"))]
    u8,
);

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
struct T5(
    #[proptest(regex(make_regex))]
    Vec<u16>,
);

// enum:

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
enum T6 {
    V0 {
        #[proptest(regex = "a+")]
        f0: (),
    }
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
enum T7 {
    V0 {
        #[proptest(regex("a*"))]
        f0: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
enum T8 {
    V0 {
        #[proptest(regex(make_regex))]
        f0: Vec<u16>,
    }
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
enum T9 {
    V0(
        #[proptest(regex = "a+")]
        (),
    )
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
enum T10 {
    V0(
        #[proptest(regex("a*"))]
        u8,
    )
}

#[derive(Debug, Arbitrary)] //~ StrategyFromRegex` is not satisfied [E0277]
enum T11 {
    V0(
        #[proptest(regex(make_regex))]
        Vec<u16>,
    )
}
