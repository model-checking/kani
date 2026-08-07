// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(rustc_attrs)]
#![allow(internal_features)]

// A ranged scalar newtype, as the deranged crate (and std's NonZero) define them: the layout
// niche IS the validity invariant.
#[rustc_layout_scalar_valid_range_start(1)]
#[rustc_layout_scalar_valid_range_end(12)]
#[derive(Clone, Copy)]
pub struct Month(u8);

impl Month {
    pub fn get(self) -> u8 {
        self.0
    }
}

pub struct Schedule {
    month: Month,
    day: u8,
}

// Previously a false alarm: raw field synthesis produced Month values outside 1..=12
// (language-level invalid), tripping the assert.
pub fn days_left_in_year(s: Schedule) -> u16 {
    assert!(s.month.get() >= 1 && s.month.get() <= 12, "invalid month is UB");
    (12 - s.month.get() as u16) * 31 + (31 - s.day.min(31) as u16)
}

// The assumption must not over-constrain: all valid months remain reachable.
pub fn cover_extremes(m: Month) {
    kani::cover!(m.get() == 1, "january reachable");
    kani::cover!(m.get() == 12, "december reachable");
}
