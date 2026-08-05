// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand verifies Debug and Display implementations.
// Their `&mut Formatter` argument cannot be generated nondeterministically; instead, the
// generated harness formats a nondeterministic value of the self type into a discarding sink
// (c.f. the CheckDebugFmt/CheckDisplayFmt models), which exercises `fmt` through the core
// formatting machinery with a real `Formatter`.
// The "TEST NOTE" comments explain the expected result per function.

// TEST NOTE: `<Percent as Debug>::fmt` should FAIL: the assert is reachable for values > 100.
#[derive(kani::Arbitrary)]
pub struct Percent(u8);

impl std::fmt::Debug for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.0 <= 100, "invalid percent");
        write!(f, "{}%", self.0)
    }
}

// TEST NOTE: `<Safe as Display>::fmt` should PASS; the self type's Arbitrary implementation
// is compiler-derived.
pub struct Safe {
    v: u32,
}

impl std::fmt::Display for Safe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Safe({})", self.v)
    }
}

// TEST NOTE: `<NotGen as Debug>::fmt` is skipped: the self type cannot be generated.
pub struct NotGen {
    p: *const u8,
}

impl std::fmt::Debug for NotGen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:p}", self.p)
    }
}
