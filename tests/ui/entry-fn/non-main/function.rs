// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --enable-unstable --mir-linker --function target_fn
//
//! Checks that we can use --function with the MIR Linker

#[kani::proof]
fn harness() {
    // Should fail if called.
    assert_eq!(1 + 1, 10);
}

#[no_mangle]
pub fn target_fn() {
    let pos: i32 = kani::any();
    kani::assume(pos > 0);
    assert!(pos != 0);
}
