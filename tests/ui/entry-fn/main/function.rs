// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness main
//
//! Checks that we can select main function as a harness.

#[kani::proof]
fn harness() {
    // Should fail if called.
    assert_eq!(1 + 1, 10);
}

#[kani::proof]
pub fn target_fn() {
    let pos: i32 = kani::any();
    kani::assume(pos > 0);
    assert!(pos != 0);
}

#[kani::proof]
fn main() {
    assert_eq!(Some(10).and_then(|v| Some(v * 2)), Some(20));
}
