// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Simple test for Strata backend

#[kani::proof]
fn test_addition() {
    let x: u32 = 5;
    let y: u32 = 10;
    let z = x + y;
    assert!(z == 15);
}

#[kani::proof]
fn test_comparison() {
    let a: i32 = 42;
    let b: i32 = 100;
    assert!(a < b);
}

#[kani::proof]
fn test_boolean() {
    let flag: bool = true;
    assert!(flag);
}
