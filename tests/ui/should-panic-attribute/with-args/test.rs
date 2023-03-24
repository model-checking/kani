// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `#[kani::should_panic]` doesn't accept arguments.

#[kani::proof]
#[kani::should_panic(arg)]
fn check() {}
