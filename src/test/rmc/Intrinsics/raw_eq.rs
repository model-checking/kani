// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
#![feature(const_intrinsic_raw_eq)]
#![deny(const_err)]

pub fn main() {
    // Check that we get the expected results for the `raw_eq` intrinsic
    use std::intrinsics::raw_eq;

    let raw_eq_i32_true: bool = unsafe { raw_eq(&42_i32, &42) };
    assert!(raw_eq_i32_true);

    let raw_eq_i32_false: bool = unsafe { raw_eq(&4_i32, &2) };
    assert!(!raw_eq_i32_false);

    let raw_eq_char_true: bool = unsafe { raw_eq(&'a', &'a') };
    assert!(raw_eq_char_true);

    let raw_eq_char_false: bool = unsafe { raw_eq(&'a', &'A') };
    assert!(!raw_eq_char_false);

    let raw_eq_array_true: bool = unsafe { raw_eq(&[13_u8, 42], &[13, 42]) };
    assert!(raw_eq_array_true);

    const raw_eq_array_false: bool = unsafe { raw_eq(&[13_u8, 42], &[42, 13]) };
    assert!(!raw_eq_array_false);
}
