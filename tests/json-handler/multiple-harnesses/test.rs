// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --export-json multi_harness_output.json

//! Test JSON export with multiple harnesses
//! Verifies that frontend correctly handles multiple verification harnesses

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn divide(a: i32, b: i32) -> i32 {
    a / b
}

#[kani::proof]
fn verify_multiply_positive() {
    let x: i32 = kani::any();
    let y: i32 = kani::any();
    kani::assume(x > 0 && x < 10);
    kani::assume(y > 0 && y < 10);
    let result = multiply(x, y);
    assert!(result > 0);
}

#[kani::proof]
fn verify_multiply_zero() {
    let x: i32 = kani::any();
    let result = multiply(x, 0);
    assert_eq!(result, 0);
}

#[kani::proof]
fn verify_divide_nonzero() {
    let x: i32 = kani::any();
    let y: i32 = kani::any();
    kani::assume(x >= 0 && x < 100);
    kani::assume(y > 0 && y < 100);
    let result = divide(x, y);
    assert!(result <= x);
}
