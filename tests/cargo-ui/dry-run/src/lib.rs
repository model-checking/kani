// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The dry run doesn't actually compile this test. Hence add some broken code.
//! In `expected` you will find substrings of these commands because the
//! concrete paths depend on your working directory.

#[kani::proof]
pub fn broken_harness() {
    let invalid: Nope = Nope{};
}
