// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test makes sure Kani handles the valid `assert!(&b)` syntax where `b` is a `bool`
//! See https://github.com/model-checking/kani/issues/2108 for details.

#[kani::proof]
fn check_assert_with_reg() {
    let b1: bool = kani::any();
    let b2 = b1 || !b1; // true
    assert!(&b2);
}
