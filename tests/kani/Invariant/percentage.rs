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

impl Percentage {
    pub fn try_new(val: u8) -> Result<Self, String> {
        if val <= 100 {
            Ok(Self(val))
        } else {
            Err(String::from("error: invalid percentage value"))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn increase(&self, other: u8) -> Percentage {
        let amount = self.0 + other;
        Percentage::try_new(amount.min(100)).unwrap()
    }
}

impl kani::Invariant for Percentage {
    fn is_safe(&self) -> bool {
        self.0 <= 100
    }
}

#[kani::proof]
fn check_assume_safe() {
    let percentage: Percentage = kani::any();
    kani::assume(percentage.is_safe());
    assert!(percentage.value() <= 100);
}

#[kani::proof]
#[kani::should_panic]
fn check_assert_safe() {
    let percentage: Percentage = kani::any();
    assert!(percentage.is_safe());
}

#[kani::proof]
fn check_increase_safe() {
    let percentage: Percentage = kani::any();
    kani::assume(percentage.is_safe());
    let amount = kani::any();
    kani::assume(amount <= 100);
    let new_percentage = percentage.increase(amount);
    assert!(new_percentage.is_safe());
}
