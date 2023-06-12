// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --fail-uncoverable

//! Checks that the `--fail-uncoverable` option cause a failure due to a cover
//! statement not being satisfied.

#[kani::proof]
fn harness_unsatisfied() {
    let x: i32 = kani::any();
    kani::assume(x >= 0);
    kani::cover!(x == -1);
}
