// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test function calls in Strata backend

fn add(a: u32, b: u32) -> u32 {
    a + b
}

fn multiply(x: u32, y: u32) -> u32 {
    x * y
}

#[kani::proof]
fn test_function_call() {
    let result = add(5, 10);
    assert!(result == 15);
}

#[kani::proof]
fn test_nested_calls() {
    let sum = add(3, 4);
    let product = multiply(sum, 2);
    assert!(product == 14);
}

#[kani::proof]
fn test_kani_any() {
    let x: u32 = kani::any();
    kani::assume(x < 100);
    assert!(x < 200);
}
