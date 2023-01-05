// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --enable-unstable --function harness
//! This test covers an issue we had with our public-fns implementation.
//! We were not checking if a root was a function in the first place.
//! https://github.com/model-checking/kani/issues/2047

pub static DAYS_OF_WEEK: [char; 7] = ['s', 'm', 't', 'w', 't', 'f', 's'];

#[no_mangle]
pub fn harness() {
    let day: usize = kani::any();
    kani::assume(day < DAYS_OF_WEEK.len());
    assert!(['s', 'm', 't', 'w', 'f'].contains(&DAYS_OF_WEEK[day]));
}
