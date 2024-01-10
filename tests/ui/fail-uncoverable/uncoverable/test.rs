// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --fail-uncoverable

//! Checks that enabling the `--fail-uncoverable` option triggers a
//! condition-based failure due to a `cover` statement not being satisfied.
//!
//! Note: The `cover` statement isn't satisfied because it's unreachable.

#[kani::proof]
fn harness_uncoverable() {
    let x: i32 = kani::any();
    kani::assume(x >= 0);
    if x < 0 {
        kani::cover!();
    }
}
