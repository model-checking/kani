// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Each harness asserts what holds for the whole allocation but not for the constant subslice or
//! reference that points into it, so every harness must fail.
//! <https://github.com/model-checking/kani/issues/4971>

const A: &[u8] = &[1, 2, 3, 4];
const S: &[u8] = A.split_at(1).1;
const T: &str = "abcd".split_at(2).1;
const U: &str = "abcd".split_at(2).0;
const R: &u8 = &A[1];

#[kani::proof]
fn slice_len() {
    assert!(S.len() == 4);
}

#[kani::proof]
fn slice_first() {
    assert!(S[0] == 1);
}

#[kani::proof]
fn str_len() {
    assert!(T.len() == 4);
}

#[kani::proof]
fn str_first() {
    assert!(T.as_bytes()[0] == b'a');
}

#[kani::proof]
fn str_prefix_len() {
    assert!(U.len() == 4);
}

#[kani::proof]
fn element_ref() {
    assert!(*R == 1);
}
