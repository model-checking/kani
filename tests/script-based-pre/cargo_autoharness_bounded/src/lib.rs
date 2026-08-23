// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports arguments whose types implement
// `BoundedArbitrary` (rather than `Arbitrary`), e.g. `Vec<T>`, `String`, or user types
// deriving it. The generated harness produces a bounded nondeterministic value via
// `kani::bounded_any` with bound AUTOHARNESS_BOUNDED_ANY_BOUND (4); verification results
// only hold up to that bound. The "TEST NOTE" comments explain the expected result per
// function.

// TEST NOTE: should PASS: summing at most 4 u8s cannot overflow u64.
pub fn vec_sum(xs: Vec<u8>) -> u64 {
    xs.iter().map(|&x| x as u64).sum()
}

// TEST NOTE: should FAIL: the vector may be empty, so the index may be out of bounds.
pub fn vec_first(xs: Vec<u8>) -> u8 {
    xs[0]
}

// TEST NOTE: should PASS: strings generated via String's BoundedArbitrary implementation.
pub fn string_head(s: String) -> Option<char> {
    s.chars().next()
}

// TEST NOTE: should FAIL: the string may be empty, so the index may be out of bounds.
pub fn string_first_byte(s: String) -> u8 {
    s.as_bytes()[0]
}

// TEST NOTE: should PASS: user-defined types deriving BoundedArbitrary are supported too.
#[derive(kani::BoundedArbitrary)]
pub struct Packet {
    #[bounded]
    payload: Vec<u8>,
    flag: bool,
}

pub fn packet_check(p: Packet) -> usize {
    if p.flag { p.payload.len() } else { 0 }
}

// TEST NOTE: should PASS, and the cover check must be SATISFIED: maximum-length vectors
// with specific nondeterministic contents are generated.
pub fn vec_cover(xs: Vec<u8>) {
    kani::cover!(xs.len() == 4 && xs[0] == 42, "max-length vec with specific contents");
}

// TEST NOTE: skipped: Vec<Vec<u8>> is unsupported, since Vec's BoundedArbitrary
// implementation requires the element type to implement Arbitrary.
pub fn nested_vec(xs: Vec<Vec<u8>>) -> usize {
    xs.len()
}
