// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Mimics time::Date: private packed field whose raw values violate the type invariant.
pub struct Day {
    value: u16, // invariant: 1..=366
}

impl Day {
    pub fn new(d: u16) -> Option<Day> {
        if d >= 1 && d <= 366 { Some(Day { value: d }) } else { None }
    }

    // Without --constructor-args, raw field synthesis reaches the debug_assert-style branch
    // below and reports a false alarm; with it, only valid Days are generated.
    pub fn ordinal0(&self) -> u16 {
        assert!(self.value >= 1, "invariant violated");
        self.value - 1
    }
}

// Direct-returning constructor case.
pub struct Celsius {
    milli: i32,
}

impl Celsius {
    pub fn from_milli(m: i32) -> Celsius {
        Celsius { milli: m }
    }
    pub fn get(&self) -> i32 {
        self.milli
    }
}

// Result-returning constructor case.
pub struct Even {
    n: u32,
}

impl Even {
    pub fn try_new(n: u32) -> Result<Even, ()> {
        if n % 2 == 0 { Ok(Even { n }) } else { Err(()) }
    }
    pub fn half(&self) -> u32 {
        assert!(self.n % 2 == 0);
        self.n / 2
    }
}

// Assert-guarded representation constructors (unsafe/doc-hidden/_unchecked) are inlined
// with their validity assertions converted into filters — including one level of nesting
// (Wrapper's ctor calls Ranged's).
pub struct Ranged {
    value: u16, // invariant 1..=366, stated by new_unchecked's debug_asserts
}

impl Ranged {
    #[doc(hidden)]
    pub const fn new_unchecked(v: u16) -> Ranged {
        debug_assert!(v >= 1);
        debug_assert!(v <= 366);
        Ranged { value: v }
    }
}

pub struct Wrapper {
    inner: Ranged,
}

impl Wrapper {
    #[doc(hidden)]
    pub const fn from_raw_unchecked(v: u16) -> Wrapper {
        Wrapper { inner: Ranged::new_unchecked(v) }
    }
}

// TEST NOTE: should PASS with --constructor-args (the nested debug_asserts filter the
// generated values); FAILS without.
pub fn wrapped_ordinal0(w: Wrapper) -> u16 {
    assert!(w.inner.value >= 1, "invariant violated");
    w.inner.value - 1
}
