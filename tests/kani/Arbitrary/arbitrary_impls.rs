// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Check that the Arbitrary implementations that we include in the kani library respect the
//! underlying types invariant.

#[kani::proof]
#[kani::unwind(4)]
fn check_any_array() {
    let arr: [bool; 2] = kani::any();
    assert!((0..=1).contains(&(arr[0] as u8)));
    assert!((0..=1).contains(&(arr[1] as u8)));
}

/// The only valid bit values for a boolean variable are 0x0 (false) and 0x1 (true).
#[kani::proof]
fn check_any_bool() {
    let b: bool = kani::any();
    match b {
        true => assert!(b as u8 == 1),
        false => assert!(b as u8 == 0),
    }
    assert!(matches!(b, true | false));
}

#[kani::proof]
fn check_any_char() {
    let c: char = kani::any();
    assert!(c <= char::MAX);
}
