// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

#[kani::proof]
pub fn check_encoding() {
    let hello = utf8_percent_encode("hello world", NON_ALPHANUMERIC);
    assert_eq!(hello.to_string(), "hello%20world");
}
