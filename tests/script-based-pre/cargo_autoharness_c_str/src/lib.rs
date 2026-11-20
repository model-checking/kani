// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports `&CStr` arguments. The generated harness
// produces a C string of nondeterministic length, bounded by AUTOHARNESS_SLICE_BOUND (16) less
// the terminating NUL, backed by nondeterministic harness-local storage whose last byte is NUL,
// c.f. the `AnyCStrRef` model. The "TEST NOTE" comments explain the expected result per function.

use std::ffi::CStr;

// TEST NOTE: should PASS: the length of a C string of at most 15 bytes fits any usize.
pub fn len(s: &CStr) -> usize {
    s.to_bytes().len()
}

// TEST NOTE: should FAIL: the C string may be empty, so the index may be out of bounds.
pub fn first(s: &CStr) -> u8 {
    s.to_bytes()[0]
}

// TEST NOTE: should PASS: the generated value is a valid C string, so no interior NUL.
pub fn no_interior_nul(s: &CStr) {
    assert!(!s.to_bytes().contains(&0));
}

// TEST NOTE: should PASS, and the cover checks must be SATISFIED: the empty C string, the
// longest C string, and a specific content are all generated.
pub fn c_str_cover(s: &CStr) {
    kani::cover!(s.is_empty(), "empty C string");
    kani::cover!(s.to_bytes().len() == 15, "maximum-length C string");
    kani::cover!(s.to_bytes() == b"ab", "C string \"ab\"");
}

// TEST NOTE: is skipped: a C string behind a further reference is not supported, as for
// slices, since the backing storage would not outlive the value.
pub fn nested(s: &&CStr) -> usize {
    s.to_bytes().len()
}
