// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate_b::*;

#[kani::proof]
fn check() {
    assert_true(true);
    assert_false(false);
}

#[kani::proof]
#[kani::stub(::crate_b::assert_true, ::crate_b::assert_false)]
#[kani::stub(assert_false, assert_true)]
fn check_inverted() {
    assert_true(false);
    assert_false(true);
}
