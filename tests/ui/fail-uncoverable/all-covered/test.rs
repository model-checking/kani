// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --fail-uncoverable

//! Checks that all cover statements are covered/satisfied and
//! `--fail-uncoverable` doesn't cause failures.

#[kani::proof]
fn harness1() {
    kani::cover!();
}

#[kani::proof]
fn harness2() {
    let x: u32 = kani::any();
    kani::cover!(x == 1);
}
