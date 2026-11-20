// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --export-json schema_validation_output.json

//! Test JSON schema structure validation
//! Ensures frontend exports well-formed JSON with correct metadata

fn calculate(x: u32, y: u32) -> u32 {
    if x > y { x - y } else { y - x }
}

#[kani::proof]
fn verify_calculate() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    kani::assume(a < 1000);
    kani::assume(b < 1000);
    let result = calculate(a, b);
    assert!(result < 1000);
}

#[kani::proof]
fn verify_calculate_same() {
    let x: u32 = kani::any();
    let result = calculate(x, x);
    assert_eq!(result, 0);
}
