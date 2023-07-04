// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Test that captures how Kani implements various redundant checks
//! for the same operation. This can be confusing for the user, since
//! the duplicated check will always succeed, even when the first check fails.
//! <https://github.com/model-checking/kani/issues/2579>

use std::hint::black_box;

#[kani::proof]
fn check_division() {
    black_box(kani::any::<i8>() / kani::any::<i8>());
}

#[kani::proof]
fn check_remainder() {
    black_box(kani::any::<i8>() % kani::any::<i8>());
}
