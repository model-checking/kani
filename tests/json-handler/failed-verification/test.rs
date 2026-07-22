// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --export-json failed_output.json

//! Test JSON export with failed verification
//! Verifies that frontend correctly captures failure information in JSON

fn unsafe_subtract(a: u32, b: u32) -> u32 {
    a - b // This will fail with overflow
}

#[kani::proof]
fn verify_unsafe_subtract() {
    let x: u32 = kani::any();
    let y: u32 = kani::any();
    // No assumptions - will trigger overflow
    let result = unsafe_subtract(x, y);
    assert!(result <= x); // This will also fail when y > x
}
