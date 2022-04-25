// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that values returned by `kani::slice::any_raw_slice` do
// *not* necessarily satisfy the type invariant

// kani-flags: --default-unwind 4

extern crate kani;
use kani::slice::{any_raw_slice, AnySlice};
use kani::Invariant;

#[kani::proof]
fn check_any_raw_slice_invalid() {
    let s: AnySlice<char, 3> = unsafe { any_raw_slice() };
    for i in s.get_slice() {
        kani::expect_fail(i.is_valid(), "any_raw_slice values may not be valid");
    }
}
