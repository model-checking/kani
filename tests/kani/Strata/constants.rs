// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test constant extraction in Strata backend

#[kani::proof]
fn test_integer_constants() {
    let a: u32 = 42;
    let b: i32 = -10;
    let c: u8 = 255;
    assert!(a == 42);
    assert!(b == -10);
    assert!(c == 255);
}

#[kani::proof]
fn test_boolean_constants() {
    let t: bool = true;
    let f: bool = false;
    assert!(t);
    assert!(!f);
}

#[kani::proof]
fn test_arithmetic_with_constants() {
    let x: u32 = 10;
    let y: u32 = 20;
    let sum = x + y;
    assert!(sum == 30);
}

#[kani::proof]
fn test_comparison_with_constants() {
    let value: i32 = 100;
    assert!(value > 50);
    assert!(value < 200);
    assert!(value == 100);
}
