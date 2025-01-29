// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Checks that we correctly fail if the harness is a generic function.

#[kani::proof]
fn harness<T: Default>() {
    let _ = T::default();
}

#[kani::proof]
fn other_harness() {
    harness::<String>();
}
