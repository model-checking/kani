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

// REGRESSION (brotli/sharded-slab sweep ICE): a constructor with its own early-bound
// lifetime (AtomicU8::from_ptr-style) must not break discovery for the type: it gets
// erased-lifetime args (and is skipped here in favor of `new`, which scores more args).
pub struct Meter {
    level: u8,
}
impl Meter {
    pub fn new(level: u8, cap: u8) -> Meter {
        Meter { level: level.min(cap) }
    }
    pub fn from_ref<'a>(r: &'a u8) -> Meter {
        Meter { level: *r }
    }
    pub fn level(&self) -> u8 {
        self.level
    }
}

// REGRESSION (regex-automata sweep ICE): impl with its own lifetime parameter FIRST and
// the ADT generic over types; argument construction must be positional against the full
// parent+own generics, not append lifetimes at the end.
pub struct Tagged<T> {
    v: T,
    tag: u16,
}
impl<'h, T: Copy> Tagged<T> {
    pub fn build(v: T, tag: u16) -> Tagged<T> {
        Tagged { v, tag }
    }
    pub fn peek(&self, _probe: &'h u8) -> u16 {
        self.tag
    }
}
pub fn use_tagged(t: Tagged<u32>) -> u32 {
    if t.tag > 0 { t.v } else { 0 }
}

// REGRESSION (async-io/js-sys/quinn-udp/wasm-bindgen sweep ICE): a candidate constructor
// whose argument carries an escaping late-bound region inside an ADT (BorrowedFd-style)
// must be rejected without panicking the trait solver.
pub struct Borrowed<'a>(pub &'a u32);
pub struct Meter2 {
    level: u32,
}
impl Meter2 {
    pub fn from_borrowed(b: Borrowed<'_>, bump: u32) -> Meter2 {
        Meter2 { level: *b.0 + bump }
    }
    pub fn zero() -> Meter2 {
        Meter2 { level: 0 }
    }
    pub fn level(&self) -> u32 {
        self.level
    }
}
