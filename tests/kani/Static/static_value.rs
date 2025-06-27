// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Ensure that every static variable has a unique address and it is correctly
//! initialized.
//! This test reproduces the issue reported in <https://github.com/model-checking/kani/issues/2455>

static VAL_2_A: u8 = 2;
static VAL_2_B: u8 = 2;
static VAL_4: u8 = 4;

#[kani::proof]
fn check_same_value_diff_address() {
    assert_eq!(VAL_2_A, VAL_2_B);
    assert_ne!(&VAL_2_A as *const u8, &VAL_2_B as *const u8);
}

#[kani::proof]
fn check_diff_value_diff_address() {
    assert_ne!(VAL_2_A, VAL_4);
    assert_ne!(&VAL_2_A as *const u8, &VAL_4 as *const u8);
}
