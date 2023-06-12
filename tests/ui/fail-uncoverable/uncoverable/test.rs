// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --fail-uncoverable

//! Checks that the `--fail-uncoverable` option cause a failure due to a cover
//! statement not being covered.

#[kani::proof]
fn harness_uncoverable() {
    let x: i32 = kani::any();
    kani::assume(x >= 0);
    if x < 0 {
        kani::cover!();
    }
}
