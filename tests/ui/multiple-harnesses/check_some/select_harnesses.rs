// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness check_first_harness --harness check_second_harness
//! Ensure that we can select multiple harnesses at a time.
#[kani::proof]
fn check_first_harness() {
    assert!(1 == 1);
}

#[kani::proof]
fn check_second_harness() {
    assert!(2 == 2);
}

/// A harness that will fail verification if it is run.
#[kani::proof]
fn ignore_third_harness() {
    assert!(3 == 2);
}
