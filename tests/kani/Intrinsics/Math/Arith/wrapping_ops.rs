// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the `wrapping_<op>` intrinsics perform wrapping arithmetic
// operations as expected and do not trigger spurious overflow checks.
//
// This test is a modified version of the examples found in
// https://doc.rust-lang.org/std/primitive.u32.html for wrapping operations
#![feature(core_intrinsics)]
use std::intrinsics::{wrapping_add, wrapping_mul, wrapping_sub};

#[kani::proof]
fn test_wrapping_add() {
    // The compiler detects overflows at compile time if we use constants so we
    // declare a nondet. variable and assume the value to avoid annotations
    let x: u32 = kani::any();
    kani::assume(x == 200);
    assert!(wrapping_add(x, 55) == 255);
    assert!(wrapping_add(x, u32::MAX) == 199);
}

#[kani::proof]
fn test_wrapping_sub() {
    let x: u32 = kani::any();
    kani::assume(x == 100);
    assert_eq!(wrapping_sub(x, u32::MAX), 101);
    assert_eq!(wrapping_sub(x, 100), 0);
}

#[kani::proof]
fn test_wrapping_mul() {
    let x: u8 = kani::any();
    kani::assume(x == 12);
    assert_eq!(wrapping_mul(10u8, x), 120);
    assert_eq!(wrapping_mul(25u8, x), 44);
}
