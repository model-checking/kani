// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness -Z c-ffi

// This test is to check Kani's error handling for missing functions.
// When the support to c-ffi is enabled, any reachable missing function definition will trigger a
// verification failure.

extern "C" {
    fn missing_function() -> u32;
}

#[kani::proof]
fn harness() {
    let x = unsafe { missing_function() };
    assert!(x == 5);
}
