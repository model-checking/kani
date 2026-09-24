// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports `&Wtf8` arguments. The generated harness
// produces a WTF-8 string of nondeterministic length, bounded by AUTOHARNESS_STRING_BOUND (4),
// backed by nondeterministic harness-local storage, c.f. the `AnyWtf8Ref` model. The
// "TEST NOTE" comments explain the expected result per function.

#![feature(wtf8_internals)]
#![allow(internal_features)]

use core::wtf8::Wtf8;

// TEST NOTE: should PASS: the length of a string of at most 4 bytes fits any usize.
pub fn len(s: &Wtf8) -> usize {
    s.len()
}

// TEST NOTE: should FAIL: the string may be empty, so the index may be out of bounds.
pub fn first(s: &Wtf8) -> u8 {
    s.as_bytes()[0]
}

// TEST NOTE: should PASS, and the cover checks must be SATISFIED: the empty string, the longest
// string, a specific content, and an unpaired surrogate are all generated.
pub fn wtf8_cover(s: &Wtf8) {
    kani::cover!(s.is_empty(), "empty string");
    kani::cover!(s.len() == 4, "maximum-length string");
    kani::cover!(s.as_bytes() == b"ab", "string \"ab\"");
    kani::cover!(s.as_bytes() == b"\xED\xA0\x80", "unpaired surrogate");
}

// TEST NOTE: is skipped: a string behind a further reference is not supported, as for slices,
// since the backing storage would not outlive the value.
pub fn nested(s: &&Wtf8) -> usize {
    s.len()
}
