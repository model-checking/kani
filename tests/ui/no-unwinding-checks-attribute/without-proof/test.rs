// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `#[kani::no_unwinding_checks]` requires `#[kani::proof]`.

#[kani::no_unwinding_checks]
fn helper() {}

#[kani::proof]
fn check() {
    helper();
}
