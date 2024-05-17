// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `Invariant` implementation behaves as expected when used on a
//! custom type.

extern crate kani;
use kani::Invariant;

// We use the default `Arbitrary` implementation, which allows values that
// shouldn't be considered safe for the `Percentage` type.
#[derive(kani::Arbitrary)]
struct Percentage(u8);

impl kani::Invariant for Percentage {
    fn is_safe(&self) -> bool {
        self.0 <= 100
    }
}

#[kani::proof]
fn check_assume_safe() {
    let percentage: Percentage = kani::any();
    kani::assume(percentage.is_safe());
    assert!(percentage.0 <= 100);
}

#[kani::proof]
#[kani::should_panic]
fn check_assert_safe() {
    let percentage: Percentage = kani::any();
    assert!(percentage.is_safe());
}
