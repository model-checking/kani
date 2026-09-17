// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand verifies Debug and Display implementations.
// Their `&mut Formatter` argument cannot be generated nondeterministically; instead, the
// generated harness formats a nondeterministic value of the self type into a discarding sink
// (c.f. the CheckDebugFmt/CheckDisplayFmt models), which exercises `fmt` through the core
// formatting machinery with a real `Formatter`.
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
