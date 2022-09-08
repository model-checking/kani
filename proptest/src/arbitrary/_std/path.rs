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

//! Arbitrary implementations for `std::path`.

use std::path::*;

// TODO: Figure out PathBuf and then Box/Rc/Box<Path>.

arbitrary!(StripPrefixError; Path::new("").strip_prefix("a").unwrap_err());

#[cfg(all(test, not(kani)))]
mod test {
    no_panic_test!(
        strip_prefix_error => StripPrefixError
    );
}
