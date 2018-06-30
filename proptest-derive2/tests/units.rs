// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code, unused_variables, unused_imports)]

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)]
struct T0;

#[derive(Debug, Arbitrary)]
struct T1 {}

#[derive(Debug, Arbitrary)]
struct T2();

#[derive(Debug, Arbitrary)]
enum T3 { V0, }

#[derive(Debug, Arbitrary)]
enum T4 { V1(), }

#[derive(Debug, Arbitrary)]
enum T5 { V2 {}, }
