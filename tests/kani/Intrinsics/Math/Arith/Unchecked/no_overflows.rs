// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that none of these operations trigger spurious overflow checks.
#![feature(core_intrinsics)]
use std::intrinsics::{
    unchecked_add, unchecked_div, unchecked_mul, unchecked_rem, unchecked_shl, unchecked_shr,
    unchecked_sub,
};

// `checked_shr` and `checked_shl` require `u32` for their argument. We use
// `u32` in those cases and `u8` for the rest because they perform better.
macro_rules! verify_no_overflow {
    ($ty:ty, $cf: ident, $uf: ident) => {{
        let a: $ty = kani::any();
        let b: $ty = kani::any();
        let checked = a.$cf(b);
        kani::assume(checked.is_some());
        let unchecked = unsafe { $uf(a, b) };
        assert!(checked.unwrap() == unchecked);
    }};
}

#[kani::proof]
fn test_unchecked_add() {
    verify_no_overflow!(u8, checked_add, unchecked_add);
}

#[kani::proof]
fn test_unchecked_sub() {
    verify_no_overflow!(u8, checked_sub, unchecked_sub);
}

#[kani::proof]
fn test_unchecked_mul() {
    verify_no_overflow!(u8, checked_mul, unchecked_mul);
}

#[kani::proof]
fn test_unchecked_div() {
    verify_no_overflow!(u8, checked_div, unchecked_div);
}

#[kani::proof]
fn test_unchecked_rem() {
    verify_no_overflow!(u8, checked_rem, unchecked_rem);
}

#[kani::proof]
fn test_unchecked_shl() {
    verify_no_overflow!(u32, checked_shl, unchecked_shl);
}

#[kani::proof]
fn test_unchecked_shr() {
    verify_no_overflow!(u32, checked_shr, unchecked_shr);
}
