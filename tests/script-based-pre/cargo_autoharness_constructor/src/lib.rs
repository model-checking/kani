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
