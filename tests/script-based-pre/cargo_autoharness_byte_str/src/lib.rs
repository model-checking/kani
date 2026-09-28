// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports `&ByteStr` arguments. The generated harness
// produces a byte string of nondeterministic length, bounded by AUTOHARNESS_SLICE_BOUND (16),
// backed by nondeterministic harness-local storage, c.f. the `AnyByteStrRef` model. The
// "TEST NOTE" comments explain the expected result per function.

#![feature(bstr)]

use std::bstr::ByteStr;

// TEST NOTE: should PASS: the length of a byte string of at most 16 bytes fits any usize.
pub fn len(s: &ByteStr) -> usize {
    s.len()
}

// TEST NOTE: should FAIL: the byte string may be empty, so the index may be out of bounds.
pub fn first(s: &ByteStr) -> u8 {
    s[0]
}

// TEST NOTE: should PASS, and the cover checks must be SATISFIED: the empty byte string, the
// longest byte string, and a specific content are all generated.
pub fn byte_str_cover(s: &ByteStr) {
    kani::cover!(s.is_empty(), "empty byte string");
    kani::cover!(s.len() == 16, "maximum-length byte string");
    kani::cover!(&*s == b"ab", "byte string \"ab\"");
}

// TEST NOTE: is skipped: a byte string behind a further reference is not supported, as for
// slices, since the backing storage would not outlive the value.
pub fn nested(s: &&ByteStr) -> usize {
    s.len()
}
