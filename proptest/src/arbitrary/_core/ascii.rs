//-
// Copyright 2017, 2018 The proptest developers
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

//! Arbitrary implementations for `std::ascii`.

use core::ascii::{escape_default, EscapeDefault};

use crate::arbitrary::*;
use crate::strategy::statics::static_map;

arbitrary!(EscapeDefault, SMapped<u8, Self>;
    static_map(any::<u8>(), escape_default));

#[cfg(all(test, not(kani)))]
mod test {
    no_panic_test!(escape_default => EscapeDefault);
}
