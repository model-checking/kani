// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --log-file verification.log
//
// This test checks that the --log-file option works correctly
// and writes verification output to the specified file

#[kani::proof]
fn test_simple_verification() {
    let x: u32 = kani::any();
    if x < 100 {
        assert!(x < 100);
    }
}

#[kani::proof]
fn test_another_harness() {
    let y: i32 = kani::any();
    assert!(y == y);
}
