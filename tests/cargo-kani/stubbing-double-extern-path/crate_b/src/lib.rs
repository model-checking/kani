// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define `assert_false` function and export `assert_true` as well.
pub use crate_a::*;

pub fn assert_false(b: bool) {
    assert!(!b);
}
