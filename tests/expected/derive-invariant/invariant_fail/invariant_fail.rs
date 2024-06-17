// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that a verification failure is triggered when the derived `Invariant`
//! method is checked but not satisfied.

extern crate kani;
use kani::Invariant;
// Note: This represents an incorrect usage of `Arbitrary` and `Invariant`.
//
// The `Arbitrary` implementation should respect the type invariant,
// but Kani does not enforce this in any way at the moment.
// <https://github.com/model-checking/kani/issues/3265>
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
