// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixture for the `--log-file` test: two harnesses, so the log has more than
//! one per-harness section to account for.

#[kani::proof]
fn check_one() {
    let x: u32 = kani::any();
    kani::assume(x < 100);
    assert!(x < 100);
}

#[kani::proof]
fn check_two() {
    let y: i32 = kani::any();
    assert!(y == y);
}
