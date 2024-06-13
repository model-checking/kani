// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that a verification failure is triggered when the derived `Invariant`
//! method is checked but not satisfied.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
struct NotNegative(i32);

impl kani::Invariant for NotNegative {
    fn is_safe(&self) -> bool {
        self.0 >= 0
    }
}

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct NotNegativeWrapper {
    x: NotNegative,
}

#[kani::proof]
fn check_invariant_fail() {
    let wrapper: NotNegativeWrapper = kani::any();
    assert!(wrapper.is_safe());
}
