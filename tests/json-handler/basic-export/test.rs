// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --export-json basic_test_output.json

//! Basic test for JSON export functionality
//! This verifies that the frontend module can export verification results

fn add_numbers(a: u32, b: u32) -> u32 {
    a + b
}

#[kani::proof]
fn verify_add_numbers() {
    let x: u32 = kani::any();
    let y: u32 = kani::any();
    kani::assume(x < 100);
    kani::assume(y < 100);
    let result = add_numbers(x, y);
    assert!(result >= x);
    assert!(result >= y);
}
