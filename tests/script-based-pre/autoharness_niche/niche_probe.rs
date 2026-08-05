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

// The motivating real-world case (found in the crates.io evaluation): `Duration`'s
// `Nanoseconds` field carries a 0..=999_999_999 niche, so raw field synthesis produced
// durations whose subsec_nanos exceeded a second.
pub fn duration_nanos(d: std::time::Duration) -> u32 {
    assert!(d.subsec_nanos() < 1_000_000_000, "nanos out of range");
    d.subsec_nanos()
}

// A *wrapping* niche: `NonZero`'s valid range is 1..=0, so the range check has to be a
// disjunction rather than a conjunction.
pub fn nonzero(v: std::num::NonZeroU8) -> u8 {
    assert!(v.get() != 0, "NonZeroU8 was zero");
    v.get()
}

// A niche reached through *generic instantiation*: the candidate type for `M` is derived from
// `Monthly`'s only implementor, so the value is generated for a niche-carrying type chosen by
// the instantiation search rather than named in the signature.
pub trait Monthly {
    fn month(&self) -> u8;
}
impl Monthly for Month {
    fn month(&self) -> u8 {
        self.0
    }
}
pub fn check_monthly<M: Monthly>(m: M) -> u8 {
    assert!(m.month() >= 1 && m.month() <= 12, "generic instantiation broke the niche");
    m.month()
}

// A niche on a *signed* scalar: the valid range is expressed as raw bit patterns, so the
// comparison must be unsigned (here start=1 with no end, i.e. every pattern but 0).
#[rustc_layout_scalar_valid_range_start(1)]
#[derive(Clone, Copy)]
pub struct NonZeroI8(i8);
pub fn signed_niche(p: NonZeroI8) -> i8 {
    assert!(p.0 != 0, "NonZeroI8 was zero");
    p.0
}
