// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! This test checks that contracts havoc static variables with interior mutability.
//! For now, we over-approximate and havoc the entire static.

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

/// The mutable field can be anything.
#[kani::ensures(|result| *result == old(unsafe { *ZERO_VAL.mut_field.get() }))]
pub unsafe fn mut_field() -> u8 {
    unsafe { *ZERO_VAL.mut_field.get() }
}

/// This harness is duplicated in `fixme_static_interior_mut.rs`.
/// Issue <>
#[cfg(fixme)]
#[kani::proof_for_contract(regular_field)]
fn check_regular_field_is_const() {
    assert_eq!(regular_field(), 0); // ** This should succeed since this field is constant.
}

// Ensure that Kani havoc the mutable field.
#[kani::should_panic]
#[kani::proof_for_contract(mut_field)]
fn check_regular_field_is_const() {
    assert_eq!(unsafe { mut_field() }, 0); // ** This must fail since Kani havoc the mutable field.
}
