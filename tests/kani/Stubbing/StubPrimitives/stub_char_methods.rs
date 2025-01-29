// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can correctly stub char functions.
/// Check that we can stub is_ascii from `char`.
pub fn stub_is_ascii_true(_: &char) -> bool {
    true
}

/// Check stubbing by directly calling `str::is_ascii`
#[kani::proof]
#[kani::stub(char::is_ascii, stub_is_ascii_true)]
pub fn check_stub_is_ascii() {
    let input: char = kani::any();
    assert!(input.is_ascii());
}
