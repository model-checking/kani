// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check the `Invariant` implementations that we include in the Kani library
//! with respect to the underlying type invariants.
extern crate kani;
use kani::Invariant;

#[kani::proof]
#[kani::should_panic]
fn check_unsafe_char() {
    let unsafe_char = unsafe { char::from_u32_unchecked(0x110000) };
    assert!(unsafe_char.is_safe());
}

#[kani::proof]
fn check_safe_char() {
    let safe_char: char = kani::any();
    assert!(safe_char.is_safe());
}

#[kani::proof]
#[kani::should_panic]
fn check_unsafe_bool() {
    let unsafe_bool: bool = unsafe { std::mem::transmute(2_u8) };
    assert!(unsafe_bool.is_safe());
}

#[kani::proof]
fn check_safe_bool() {
    let safe_bool: bool = kani::any();
    assert!(safe_bool.is_safe());
}

macro_rules! check_safe_type {
    ( $type: ty ) => {
        let value: $type = kani::any();
        assert!(value.is_safe());
    };
}

#[kani::proof]
fn check_safe_impls() {
    check_safe_type!(u8);
    check_safe_type!(u16);
    check_safe_type!(u32);
    check_safe_type!(u64);
    check_safe_type!(u128);
    check_safe_type!(usize);

    check_safe_type!(i8);
    check_safe_type!(i16);
    check_safe_type!(i32);
    check_safe_type!(i64);
    check_safe_type!(i128);
    check_safe_type!(isize);

    check_safe_type!(f32);
    check_safe_type!(f64);

    check_safe_type!(());
}
