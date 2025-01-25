// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks -Z mem-predicates
//! Check that Kani can correctly identify value validity of `char` and structures with `char`.
//! Note that we use `black_box` hint to ensure the logic doesn't get removed as dead code.

use std::num::NonZeroU32;

#[repr(C)]
#[derive(Copy, Clone, kani::Arbitrary)]
struct OneField<T>(T);

#[repr(C)]
#[derive(Copy, Clone, kani::Arbitrary)]
struct TwoFields<T, U>(T, U);

#[repr(C)]
#[derive(Copy, Clone, kani::Arbitrary)]
struct ThreeFields<T, U, V>(T, U, V);

/// Check that valid u32's are all identified as valid.
#[kani::proof]
fn check_char_ok() {
    let val = kani::any_where(|v: &u32| char::from_u32(*v).is_some());
    assert!(kani::mem::can_dereference(&val as *const _ as *const char));
    let c1: char = unsafe { std::mem::transmute(val) };
    let c2 = unsafe { char::from_u32_unchecked(val) };
    let c3 = char::from_u32(val).unwrap();
    assert_eq!(c1, c2);
    assert_eq!(c2, c3);
}

/// Check that all invalid u32's identified as invalid.
#[kani::proof]
fn cannot_dereference_invalid_char() {
    let val = kani::any_where(|v: &u32| char::from_u32(*v).is_none());
    assert!(!kani::mem::can_dereference(&val as *const _ as *const char));
}

/// Check that transmuting from invalid u32's trigger a UB check.
#[kani::proof]
fn check_invalid_char_should_fail() {
    let val = kani::any_where(|v: &u32| char::from_u32(*v).is_none());
    let _ = if kani::any() {
        unsafe { char::from_u32_unchecked(val) }
    } else {
        unsafe { std::mem::transmute(val) }
    };
    assert!(false, "Unreachable code: Expected invalid char detection");
}

#[kani::proof]
fn check_valid_char_wrappers() {
    let v1 = kani::any_where(|v: &u32| char::from_u32(*v).is_some());
    let v2 = kani::any_where(|v: &u32| char::from_u32(*v).is_some());
    let v3 = kani::any_where(|v: &u32| char::from_u32(*v).is_some());
    assert!(kani::mem::can_dereference(&OneField(v1) as *const _ as *const OneField<char>));
    assert!(kani::mem::can_dereference(
        &TwoFields(v1, v2) as *const _ as *const TwoFields<char, char>
    ));
    assert!(kani::mem::can_dereference(
        &ThreeFields(v1, v2, v3) as *const _ as *const ThreeFields<char, char, char>
    ));
}

/// Ensure that we correctly identify validity of a structure with fields with different
/// requirements.
#[kani::proof]
fn check_valid_mixed_wrapper() {
    let unicode = kani::any_where(|v: &u32| char::from_u32(*v).is_some());
    let non_zero = kani::any_where(|v: &u32| *v != 0);
    assert!(kani::mem::can_dereference(
        &TwoFields(unicode, non_zero) as *const _ as *const TwoFields<char, NonZeroU32>
    ));
    assert!(kani::mem::can_dereference(
        &TwoFields(non_zero, unicode) as *const _ as *const TwoFields<NonZeroU32, char>
    ));
    assert!(kani::mem::can_dereference(
        &TwoFields((), unicode) as *const _ as *const TwoFields<(), char>
    ));
}

/// Check that transmuting from invalid wrappers trigger UB check failure.
#[kani::proof]
fn check_invalid_char_nonzero_wrapper_should_fail() {
    let unicode = kani::any_where(|v: &u32| char::from_u32(*v).is_some());
    let non_unicode = kani::any_where(|v: &u32| char::from_u32(*v).is_none());
    let non_zero = kani::any_where(|v: &u32| *v != 0);
    let var: TwoFields<char, NonZeroU32> = if kani::any() {
        unsafe { std::mem::transmute(TwoFields(non_unicode, non_zero)) }
    } else {
        unsafe { std::mem::transmute(TwoFields(unicode, 0)) }
    };
    // Ensure the condition above does not get pruned.
    std::hint::black_box(var);
    assert!(false, "Unreachable code: Expected invalid char / NonZero detection");
}

/// Check that transmuting from invalid wrappers trigger UB check failure independent
/// on the position of the unit field.
#[kani::proof]
fn check_invalid_char_unit_wrapper_should_fail() {
    let non_unicode = kani::any_where(|v: &u32| char::from_u32(*v).is_none());
    if kani::any() {
        let var: TwoFields<char, ()> = unsafe { std::mem::transmute(TwoFields(non_unicode, ())) };
        std::hint::black_box(var);
    } else {
        let var: TwoFields<(), char> = unsafe { std::mem::transmute(TwoFields((), non_unicode)) };
        std::hint::black_box(var);
    }
    assert!(false, "Unreachable code: Expected invalid char wrapper detection");
}
