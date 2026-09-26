// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports `&Formatter` and `&mut Formatter` arguments. The
// generated harness builds a formatter over a sink that discards its output, with nondeterministic
// options whose width and precision are bounded by AUTOHARNESS_SLICE_BOUND (16), c.f. the
// `AnyFormatter` model. The "TEST NOTE" comments explain the expected result per function.

use std::fmt::{self, Alignment, Formatter};

// TEST NOTE: should PASS: padding a literal honours any width, fill and alignment.
pub fn pad(f: &mut Formatter<'_>) -> fmt::Result {
    f.pad("ab")
}

// TEST NOTE: should PASS, and the cover checks must be SATISFIED: each option is reachable.
pub fn options_cover(f: &Formatter<'_>) {
    kani::cover!(f.width().is_none(), "no width");
    kani::cover!(f.width() == Some(16), "maximum width");
    kani::cover!(f.align() == Some(Alignment::Center), "center alignment");
    kani::cover!(f.alternate() && f.sign_plus(), "alternate with plus sign");
    kani::cover!(f.fill() == '*', "fill '*'");
}

// TEST NOTE: should FAIL: a width or precision may be set.
pub fn assumes_default(f: &Formatter<'_>) -> bool {
    assert!(f.width().is_none() && f.precision().is_none());
    f.alternate()
}

// TEST NOTE: is skipped: a formatter behind a further reference is not supported, as for
// slices, since the backing sink would not outlive the value.
pub fn nested(f: &&Formatter<'_>) -> bool {
    f.alternate()
}
