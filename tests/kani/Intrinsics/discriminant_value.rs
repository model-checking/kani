// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `discriminant_value` returns the expected results
// for different cases
#![feature(core_intrinsics)]
use std::intrinsics::discriminant_value;

// A standard enum with variants containing fields
enum MyError {
    Error1(i32),
    Error2(&'static str),
    Error3 { description: String, code: u32 },
}

#[kani::proof]
fn test_standard_enum() {
    // Check that the values go from 0 to `num_variants - 1`
    assert!(discriminant_value(&MyError::Error1) == 0);
    assert!(discriminant_value(&MyError::Error2("bar")) == 1);
    assert!(
        discriminant_value(&MyError::Error3 { description: "some_error".to_string(), code: 3 })
            == 2
    );
}

// An enum that assigns constant values to some variants
enum Constants {
    A = 2,
    B = 5,
    C,
}

#[kani::proof]
fn test_constants_enum() {
    // Check that the values are equal to the constants assigned
    assert!(discriminant_value(&Ordering::Less) == -1);
    assert!(discriminant_value(&Ordering::Equal) == 0);
    assert!(discriminant_value(&Ordering::Greater) == 1);
}

// An enum that assigns constant values (one of them negative) to all variants
enum Ordering {
    Less = -1,
    Equal = 0,
    Greater = 1,
}

#[kani::proof]
fn test_ordering_enum() {
    // Check that the values are equal to the constants assigned
    // and the non-assigned value follows from the assigned ones
    assert!(discriminant_value(&Constants::A) == 2);
    assert!(discriminant_value(&Constants::B) == 5);
    assert!(discriminant_value(&Constants::C) == 6);
}

#[kani::proof]
fn test_no_enum() {
    // Check that the value is 0 if the type has no discriminant
    assert!(discriminant_value(&2) == 0);
}
