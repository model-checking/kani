// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `#[kani::no_unwinding_checks]` doesn't accept arguments.

#[kani::proof]
#[kani::no_unwinding_checks(arg)]
fn check() {}
