// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! This file is a duplicate of `static_interior_mut.rs` that captures the current over-approx
//! we perform for static variables with `UnsafeCell`.
//! When you fix this, please delete this file and enable the `regular_field` harness in the
//! original file.

extern crate kani;

use std::cell::UnsafeCell;

pub struct WithMut {
    regular_field: u8,
    mut_field: UnsafeCell<u8>,
}

/// Just for test purpose.
unsafe impl Sync for WithMut {}

/// A static definition of `WithMut`
static ZERO_VAL: WithMut = WithMut { regular_field: 0, mut_field: UnsafeCell::new(0) };

/// The regular field should be 0.
#[kani::ensures(|result| *result == 0)]
pub fn regular_field() -> u8 {
    ZERO_VAL.regular_field
}

/// This harness is a copy from `static_interior_mut.rs`.
/// Once this gets fixed, please delete this file and enable the original one.
#[kani::proof_for_contract(regular_field)]
fn check_regular_field_is_const() {
    assert_eq!(regular_field(), 0); // ** This should succeed since this field is constant.
}
