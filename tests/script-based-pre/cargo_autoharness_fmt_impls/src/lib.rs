// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand verifies formatting trait implementations.
// Their `&mut Formatter` argument cannot be generated nondeterministically; instead, the
// generated harness formats a nondeterministic value of the self type into a discarding sink
// (c.f. the `check_*_fmt` models), which exercises `fmt` through the core formatting
// machinery with a real `Formatter`.
// The "TEST NOTE" comments explain the expected result per function.

use std::fmt;

// TEST NOTE: `<Percent as Debug>::fmt` should FAIL: the assert is reachable for values > 100.
#[derive(kani::Arbitrary)]
pub struct Percent(u8);

impl fmt::Debug for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 <= 100, "invalid percent");
        write!(f, "{}%", self.0)
    }
}

// TEST NOTE: `<Level as Display>::fmt` should FAIL: the assert is reachable for values > 3.
// This is the `Display` counterpart of `Percent`: a passing harness would not show that the
// `Display` model (and the corresponding compiler branch) actually reaches the implementation,
// since the other `Display` cases below either have no failing property (`Safe`) or have one
// that their type invariant rules out (`Even`).
pub struct Level(u8);

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 <= 3, "invalid level");
        write!(f, "L{}", self.0)
    }
}

// TEST NOTE: `<Safe as Display>::fmt` should PASS; the self type's Arbitrary implementation
// is compiler-derived.
pub struct Safe {
    v: u32,
}

impl fmt::Display for Safe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Safe({})", self.v)
    }
}

// TEST NOTE: `<Even as Display>::fmt` should PASS: the self type implements `Invariant`, which
// the generated value is assumed to satisfy, so the assert is unreachable.
pub struct Even(u8);

impl kani::Invariant for Even {
    fn is_safe(&self) -> bool {
        self.0 % 2 == 0
    }
}

impl fmt::Display for Even {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 % 2 == 0, "odd Even");
        write!(f, "{}", self.0)
    }
}

// TEST NOTE: `<Derived as Debug>::fmt` should PASS. Compiler-derived implementations are
// verified just like hand-written ones, consistent with how autoharness treats other derived
// trait implementations (e.g. `Clone::clone` or `PartialEq::eq`).
#[derive(Debug)]
pub struct Derived {
    a: bool,
}

// TEST NOTE: `<NotGen as Debug>::fmt` is skipped: the self type cannot be generated.
pub struct NotGen {
    p: *const u8,
}

impl fmt::Debug for NotGen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:p}", self.p)
    }
}

// TEST NOTE: `<Borrowed<'_> as Debug>::fmt` is skipped: a self type with a lifetime parameter
// cannot be generated.
pub struct Borrowed<'a>(&'a u8);

impl fmt::Debug for Borrowed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Borrowed({})", self.0)
    }
}

// TEST NOTE: `<Contracted as Debug>::fmt` is skipped: a `fmt` method under contract is handled
// by the regular automatic contract harness path, which needs to call the function directly and
// therefore cannot generate the `&mut Formatter` argument.
pub struct Contracted(u8);

impl fmt::Debug for Contracted {
    #[kani::requires(self.0 <= 100)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// TEST NOTE: each of the remaining formatting traits should FAIL on its type's own assert. Each
// type implements exactly one formatting trait, so a model dispatched to the wrong trait cannot
// resolve for that type; the harness would fail to generate rather than pass.
pub struct Bin(u8);

impl fmt::Binary for Bin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 != 2, "binary");
        write!(f, "{:b}", self.0)
    }
}

pub struct Oct(u8);

impl fmt::Octal for Oct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 != 8, "octal");
        write!(f, "{:o}", self.0)
    }
}

pub struct LowHex(u8);

impl fmt::LowerHex for LowHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 != 0xa, "lower hex");
        write!(f, "{:x}", self.0)
    }
}

pub struct UpHex(u8);

impl fmt::UpperHex for UpHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 != 0xB, "upper hex");
        write!(f, "{:X}", self.0)
    }
}

pub struct LowExp(u8);

impl fmt::LowerExp for LowExp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 != 1, "lower exp");
        write!(f, "{:e}", self.0)
    }
}

pub struct UpExp(u8);

impl fmt::UpperExp for UpExp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 != 3, "upper exp");
        write!(f, "{:E}", self.0)
    }
}

pub struct Ptr(u8);

impl fmt::Pointer for Ptr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 != 7, "pointer");
        fmt::Pointer::fmt(&(self as *const Ptr), f)
    }
}
