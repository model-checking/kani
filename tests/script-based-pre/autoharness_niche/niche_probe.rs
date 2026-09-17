// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ranged scalar newtypes are expressed with pattern types since nightly-2026-06-01 removed
// `rustc_layout_scalar_valid_range_start`/`_end`; `core::num::niche_types` made the same move.
//
// Note the consequence for autoharness: a pattern type is not an ADT and has no `Arbitrary`
// implementation, so `can_derive_arbitrary` cannot synthesize a struct that has one as a field.
// The locally-defined ranged types below are therefore *skipped* rather than harnessed, which the
// expected output pins. The niche assumption itself is still exercised end to end through
// `std::time::Duration`, whose `Nanoseconds` field carries the same kind of range. Teaching
// autoharness to generate pattern-type fields (generate the base integer, assume the layout
// niche that `scalar_niche` already computes) would restore the wider reach.
#![feature(pattern_types)]
#![feature(pattern_type_macro)]

// A ranged scalar newtype, as the deranged crate (and std's NonZero) define them: the layout
// niche IS the validity invariant.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Month(std::pat::pattern_type!(u8 is 1..=12));

impl Month {
    pub fn get(self) -> u8 {
        unsafe { std::mem::transmute(self.0) }
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
        self.get()
    }
}
pub fn check_monthly<M: Monthly>(m: M) -> u8 {
    assert!(m.month() >= 1 && m.month() <= 12, "generic instantiation broke the niche");
    m.month()
}

// A niche on a *signed* scalar. The valid range is expressed as raw bit patterns, so the
// comparison must be unsigned: 1..=100 excludes 0 as well as every negative value, whose raw
// patterns (128..=255) are numerically *above* the range's end.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PosI8(std::pat::pattern_type!(i8 is 1..=100));
pub fn signed_niche(p: PosI8) -> i8 {
    let v: i8 = unsafe { std::mem::transmute(p.0) };
    assert!(v >= 1 && v <= 100, "PosI8 out of range");
    v
}
