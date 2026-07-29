// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports slice reference (`&[T]`, `&mut [T]`) and
// string slice (`&str`) arguments. The generated harness produces a slice of nondeterministic
// length, bounded by AUTOHARNESS_SLICE_BOUND (16) for slices and AUTOHARNESS_STR_BOUND (8)
// for strings, backed by nondeterministic harness-local storage, c.f. the `AnySliceRef` and
// `AnyStrRef` models. The "TEST NOTE" comments explain the expected result per function.

// TEST NOTE: should PASS: summing at most 16 u32s cannot overflow u64.
pub fn sum(xs: &[u32]) -> u64 {
    xs.iter().map(|&x| x as u64).sum()
}

// TEST NOTE: should FAIL: the slice may be empty, so the index may be out of bounds.
pub fn first(xs: &[u8]) -> u8 {
    xs[0]
}

// TEST NOTE: should PASS: writing through a mutable slice.
pub fn zero_all(xs: &mut [u8]) {
    for x in xs.iter_mut() {
        *x = 0;
    }
}

// TEST NOTE: should PASS, and the cover checks must be SATISFIED: nonempty slices of
// nondeterministic contents and length are generated.
pub fn slice_cover(xs: &[u8]) {
    kani::cover!(xs.len() == 16 && xs[0] == 255, "maximum-length slice with nondet contents");
    kani::cover!(xs.is_empty(), "empty slice");
}

// TEST NOTE: should PASS: iterating over the chars of a nondeterministic string.
pub fn count_a(s: &str) -> usize {
    s.chars().filter(|&c| c == 'a').count()
}

// TEST NOTE: should FAIL: the string may be empty, so the index may be out of bounds.
pub fn first_byte(s: &str) -> u8 {
    s.as_bytes()[0]
}

// TEST NOTE: should PASS, and the cover checks must be SATISFIED: nonempty strings with
// specific contents are generated, including multi-byte (non-ASCII) characters.
pub fn str_cover(s: &str) {
    kani::cover!(s.len() == 3 && s.starts_with('a'), "3-byte string starting with 'a'");
    kani::cover!(s.chars().next() == Some('\u{e9}'), "string starting with a non-ASCII char");
}

// TEST NOTE: skipped: nested slice references are not supported (yet).
pub fn nested(xs: &&[u8]) -> usize {
    xs.len()
}
