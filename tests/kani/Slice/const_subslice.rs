// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that constants that point partway into another constant (subslices, substrings and
//! element references) have their own data and length, not those of the whole allocation.
//! <https://github.com/model-checking/kani/issues/4971>

const A: &[u8] = &[1, 2, 3, 4];
const SPLIT_AT: &[u8] = A.split_at(1).1;
const SPLIT_AT_PREFIX: &[u8] = A.split_at(3).0;
const SPLIT_FIRST: &[u8] = match A.split_first() {
    Some((_, rest)) => rest,
    None => &[],
};
const PATTERN: &[u8] = match A {
    [_, rest @ ..] => rest,
    [] => &[],
};
const STR: &str = "abcd".split_at(2).1;
const STR_PREFIX: &str = "abcd".split_at(2).0;
const ELEMENT: &u8 = &A[1];
// Elements wider than a byte, so that a byte offset and an element index differ.
const WIDE: &[u32] = &[10, 20, 30, 40];
const WIDE_SPLIT_AT: &[u32] = WIDE.split_at(1).1;
const WIDE_ELEMENT: &u32 = &WIDE[2];

#[kani::proof]
fn check_split_at() {
    assert_eq!(SPLIT_AT.len(), 3);
    assert_eq!(SPLIT_AT[0], 2);
    assert_eq!(SPLIT_AT.last(), Some(&4));
    assert_eq!(SPLIT_AT, [2, 3, 4]);
}

#[kani::proof]
fn check_split_at_prefix() {
    assert_eq!(SPLIT_AT_PREFIX.len(), 3);
    assert_eq!(SPLIT_AT_PREFIX[0], 1);
    assert_eq!(SPLIT_AT_PREFIX.last(), Some(&3));
    assert_eq!(SPLIT_AT_PREFIX, [1, 2, 3]);
}

#[kani::proof]
fn check_split_first() {
    assert_eq!(SPLIT_FIRST.len(), 3);
    assert_eq!(SPLIT_FIRST[0], 2);
    assert_eq!(SPLIT_FIRST.last(), Some(&4));
    assert_eq!(SPLIT_FIRST, [2, 3, 4]);
}

#[kani::proof]
fn check_slice_pattern() {
    assert_eq!(PATTERN.len(), 3);
    assert_eq!(PATTERN[0], 2);
    assert_eq!(PATTERN.last(), Some(&4));
    assert_eq!(PATTERN, [2, 3, 4]);
}

#[kani::proof]
fn check_str_split_at() {
    assert_eq!(STR.len(), 2);
    assert_eq!(STR.as_bytes()[0], b'c');
    assert_eq!(STR.as_bytes().last(), Some(&b'd'));
    assert_eq!(STR, "cd");
}

#[kani::proof]
fn check_str_split_at_prefix() {
    assert_eq!(STR_PREFIX.len(), 2);
    assert_eq!(STR_PREFIX.as_bytes()[0], b'a');
    assert_eq!(STR_PREFIX.as_bytes().last(), Some(&b'b'));
    assert_eq!(STR_PREFIX, "ab");
}

#[kani::proof]
fn check_wide_split_at() {
    assert_eq!(WIDE_SPLIT_AT.len(), 3);
    assert_eq!(WIDE_SPLIT_AT[0], 20);
    assert_eq!(WIDE_SPLIT_AT.last(), Some(&40));
    assert_eq!(WIDE_SPLIT_AT, [20, 30, 40]);
}

#[kani::proof]
fn check_element_ref() {
    assert_eq!(*ELEMENT, 2);
    assert_eq!(*WIDE_ELEMENT, 30);
}

// A symbolic index keeps rustc from folding the reads into constants at higher MIR opt levels.
#[kani::proof]
fn check_symbolic_index() {
    let i: usize = kani::any();
    kani::assume(i < SPLIT_AT.len());
    assert_eq!(SPLIT_AT[i], A[i + 1]);
    let j: usize = kani::any();
    kani::assume(j < STR.len());
    assert_eq!(STR.as_bytes()[j], "abcd".as_bytes()[j + 2]);
}
