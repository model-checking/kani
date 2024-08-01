// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `#[safety_constraint(...)]` attribute works as expected when
//! deriving `Arbitrary` and `Invariant` implementations.

//! In this case, we test the attribute on a struct that represents a hybrid
//! grade (letter-numerical) which should keep the following equivalences:
//!  - A for 90-100%
//!  - B for 80-89%
//!  - C for 70-79%
//!  - D for 60-69%
//!  - F for 0-59%
//!
//! In addition, we explicitly test that `percentage` is 0-100%

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
#[safety_constraint((*letter == 'A' && *percentage >= 90 && *percentage <= 100) ||
                    (*letter == 'B' && *percentage >= 80 && *percentage < 90) ||
                    (*letter == 'C' && *percentage >= 70 && *percentage < 80) ||
                    (*letter == 'D' && *percentage >= 60 && *percentage < 70) ||
                    (*letter == 'F' && *percentage < 60))]
struct Grade {
    letter: char,
    percentage: u32,
}

impl Grade {
    pub fn check_percentage_safety(&self) {
        assert!(self.percentage <= 100);
    }
}

#[kani::proof]
fn check_grade_safe() {
    let grade: Grade = kani::any();
    assert!(grade.is_safe());
    grade.check_percentage_safety();
}
