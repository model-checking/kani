// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Ensures a harness can access a static variable.

pub static DAYS_OF_WEEK: [char; 7] = ['s', 'm', 't', 'w', 't', 'f', 's'];

#[kani::proof]
pub fn harness() {
    let day: usize = kani::any();
    kani::assume(day < DAYS_OF_WEEK.len());
    assert!(['s', 'm', 't', 'w', 'f'].contains(&DAYS_OF_WEEK[day]));
}
