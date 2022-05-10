// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check niche optimization for mix of option, tuple and nonnull.

use std::mem::{discriminant, size_of_val};
use std::num::NonZeroU128;

fn create() -> Option<NonZeroU128> {
    unsafe { Some(NonZeroU128::new_unchecked(120u128.into())) }
}

#[kani::proof]
fn check_option_128bits() {
    let opt = create();
    assert!(opt.is_some());
    assert_eq!(size_of_val(&opt), size_of_val(&opt.unwrap()));
}
