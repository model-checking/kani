//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate bit_set;
#[macro_use] extern crate quick_error;
extern crate rand;
extern crate regex_syntax;

#[cfg(test)] extern crate regex;

pub mod test_runner;
pub mod strategy;
pub mod bool;
pub mod num;
pub mod tuple;
pub mod array;
pub mod collection;
pub mod char;
pub mod string;
