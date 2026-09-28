// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Pattern types (`RigidTy::Pat`) wrap a base type with a validity constraint, e.g.
// `pattern_type!(u8 is 0..=100)`. std builds its niche types on them (`NonZero`'s inner
// type, `Duration`'s nanoseconds, wtf8 code points, ...). Autoharness must derive
// Arbitrary for integer-based pattern types -- both as struct fields and as top-level
// arguments -- and constrain the generated values to the pattern's range.
#![feature(pattern_types, pattern_type_macro)]
#![allow(internal_features)]
use std::pat::pattern_type;

pub struct Percent {
    value: pattern_type!(u8 is 0..=100),
    tag: u8,
}

// Field position: `Percent` is derived through a synthesized `any()`. The generated
// value must respect the range (assert) and both bounds must be reachable (cover).
pub fn percent_in_range(p: Percent) {
    // SAFETY: a pattern type is layout-compatible with its base type.
    let v: u8 = unsafe { std::mem::transmute(p.value) };
    kani::assert(v <= 100, "generated value must be within the pattern's range");
    kani::cover!(v == 0 && p.tag == 0, "lower bound reachable");
    kani::cover!(v == 100, "upper bound reachable");
}

// Top-level argument position.
pub fn nonzero_arg(x: pattern_type!(u8 is 1..)) {
    // SAFETY: as above.
    let v: u8 = unsafe { std::mem::transmute(x) };
    kani::assert(v != 0, "generated value must be within the pattern's range");
    kani::cover!(v == 255, "upper bound reachable");
}
