// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `#[kani::no_unwinding_checks]` can only be used once.

#[kani::proof]
#[kani::no_unwinding_checks]
#[kani::no_unwinding_checks]
fn check() {}
