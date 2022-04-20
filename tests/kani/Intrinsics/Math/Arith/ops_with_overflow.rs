// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Checks that `<op>_with_overflow` returns the expected result in all cases.
#![feature(core_intrinsics)]
use std::intrinsics::{add_with_overflow, mul_with_overflow, sub_with_overflow};

// The value of the overflow flag should match the option value returned by the
// corresponding checked operation. In other words, if `checked.is_some()` is
// assumed then `<op>_with_overflow` should not have overflown and `overflow`
// should be false. The same can be done to verify overflows with
// `checked.is_none()`.
macro_rules! verify_no_overflow {
    ($ty:ty, $cf: ident, $fwo: ident) => {{
        let a: $ty = kani::any();
        let b: $ty = kani::any();
        let checked = a.$cf(b);
        kani::assume(checked.is_some());
        let (res, overflow) = $fwo(a, b);
        assert!(!overflow);
        assert!(checked.unwrap() == res);
    }};
}

macro_rules! verify_overflow {
    ($ty:ty, $cf: ident, $fwo: ident) => {{
        let a: $ty = kani::any();
        let b: $ty = kani::any();
        let checked = a.$cf(b);
        kani::assume(checked.is_none());
        let (_res, overflow) = $fwo(a, b);
        assert!(overflow);
    }};
}

#[kani::proof]
fn test_add_with_overflow() {
    verify_no_overflow!(u8, checked_add, add_with_overflow);
    verify_overflow!(u8, checked_add, add_with_overflow);
}

#[kani::proof]
fn test_sub_with_overflow() {
    verify_no_overflow!(u8, checked_sub, sub_with_overflow);
    verify_overflow!(u8, checked_sub, sub_with_overflow);
}

#[kani::proof]
fn test_mul_with_overflow() {
    verify_no_overflow!(u8, checked_mul, mul_with_overflow);
    verify_overflow!(u8, checked_mul, mul_with_overflow);
}
