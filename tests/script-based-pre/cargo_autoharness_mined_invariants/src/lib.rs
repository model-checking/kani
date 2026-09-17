// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A type whose invariant (value in 1..=366) is stated by asserts in TWO methods:
// mined as an invariant (frequency filter passes) and assumed for generated values.
pub struct Day {
    value: u16,
}

impl Day {
    pub fn ordinal0(&self) -> u16 {
        assert!(self.value >= 1);
        self.value - 1
    }

    pub fn ordinal(&self) -> u16 {
        assert!(self.value >= 1);
        self.value
    }
}

// TEST NOTE: previously a false alarm (raw field synthesis generates value == 0);
// with mined-invariant assumption, PASSES.
pub fn day_user(d: Day) -> u16 {
    d.ordinal0()
}

// A method-local precondition asserted in only ONE method: must NOT be mined
// (frequency filter), so the false alarm on prec_user remains — honest behavior.
pub struct Gauge {
    level: u8,
}

impl Gauge {
    pub fn drain(&self) -> u8 {
        assert!(self.level >= 10, "drain requires level >= 10");
        self.level - 10
    }
}

// TEST NOTE: still FAILS (assert in drain is not mined as an invariant).
pub fn prec_user(g: Gauge) -> u8 {
    g.drain()
}

// TEST NOTE (--check-invariants): makes an INVALID Day (value == 0) — the mined-invariant
// output check must FAIL on this function.
pub fn buggy_make_day(seed: u16) -> Day {
    Day { value: seed % 366 } // BUG: yields 0 when seed % 366 == 0; invariant needs 1..=366
}

// TEST NOTE (--check-invariants): correct producer — output check must PASS.
pub fn good_make_day(seed: u16) -> Day {
    Day { value: (seed % 366) + 1 }
}

// --- V2 cases ---

// Getter-based invariant: the assert goes through self.level() (pure getter) — with
// one-level getter inlining, this mines like a direct field read (2 methods → invariant).
pub struct Tank {
    level: u8,
}

impl Tank {
    pub fn level(&self) -> u8 {
        self.level
    }
    pub fn a(&self) -> u8 {
        assert!(self.level() <= 100);
        self.level
    }
    pub fn b(&self) -> u8 {
        assert!(self.level() <= 100);
        100 - self.level
    }
}

// TEST NOTE: previously a false alarm; with getter-inlined mining, PASSES.
pub fn tank_user(t: Tank) -> u8 {
    t.b()
}

// Result-returning producer: the mined Day invariant must be checked on the Ok payload;
// this buggy producer FAILS; Err returns pass vacuously.
pub fn buggy_try_make_day(seed: u16) -> Result<Day, ()> {
    if seed > 1000 { Err(()) } else { Ok(Day { value: seed % 366 }) }
}

// TEST NOTE: correct Result producer — Ok payload valid, Err returns vacuously pass.
pub fn good_try_make_day(seed: u16) -> Result<Day, ()> {
    if seed > 1000 { Err(()) } else { Ok(Day { value: (seed % 366) + 1 }) }
}

// Enum whose variant invariant (Cm value <= 100) is asserted in two methods via match:
// mined as a variant-guarded conjunct.
pub enum Length {
    Cm(u8),
    Inch(u8),
}

impl Length {
    pub fn cm_a(&self) -> u8 {
        match self {
            Length::Cm(v) => {
                assert!(*v <= 100);
                *v
            }
            Length::Inch(v) => *v,
        }
    }
    pub fn cm_b(&self) -> u8 {
        match self {
            Length::Cm(v) => {
                assert!(*v <= 100);
                100 - *v
            }
            Length::Inch(v) => *v,
        }
    }
}

// TEST NOTE: previously a false alarm (generated Cm values above 100); with the
// variant-guarded mined conjunct assumed, PASSES.
pub fn length_user(l: Length) -> u8 {
    l.cm_b()
}
