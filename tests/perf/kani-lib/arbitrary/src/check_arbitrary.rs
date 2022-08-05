// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains performance checks for Arbitrary implementations that are included in the
//! kani library.

#[kani::proof]
fn check_any_char() {
    for _ in 0..100 {
        let c: char = kani::any();
        let n = c as u32;
        assert!(n <= 0x10FFFF);
    }

    for _ in 0..100 {
        let c: char = kani::any();
        kani::assume(c.is_digit(10));
        assert!(c.to_digit(10).is_some());
    }
}

#[kani::proof]
#[kani::unwind(101)]
fn check_any_char_array() {
    let arr: [char; 100] = kani::any();
    for i in 0..100 {
        let c = arr[i];
        let n = c as u32;
        assert!(n <= 0x10FFFF);
    }
}

#[kani::proof]
#[kani::unwind(101)]
fn check_any_usize_array() {
    let arr: [usize; 100] = kani::any();
    for i in 0..100 {
        let us = arr[i];
        kani::assume(us < 100);
        assert!(us < 1000);
    }
}

#[kani::proof]
fn check_any_usize_option() {
    let mut all_none = true;
    let mut all_some = true;
    for _ in 0..100 {
        let us: Option<usize> = kani::any();
        all_none &= us.is_none();
        all_some &= us.is_some();
    }

    assert!(!all_none || !all_some);
}

#[kani::proof]
fn check_any_usize_result() {
    let mut all_ok = true;
    let mut all_err = true;
    for _ in 0..100 {
        let us: Result<usize, isize> = kani::any();
        all_ok &= us.is_ok();
        all_err &= us.is_err();
    }

    assert!(!all_ok || !all_err);
}

#[kani::proof]
fn check_any_bool() {
    let mut all_true = true;
    let mut all_false = true;
    for _ in 0..100 {
        let val: bool = kani::any();
        all_true &= val;
        all_false &= !val;
    }

    assert!(!all_true || !all_false);
}
