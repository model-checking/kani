// Copyright 2019 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[macro_use]
extern crate proptest_derive;
#[macro_use]
extern crate proptest;

use proptest::prelude::*;

#[derive(Arbitrary, Debug)]
struct T0 {
    #[no_params] //~ ERROR: [E0658]
    field: usize,
}
