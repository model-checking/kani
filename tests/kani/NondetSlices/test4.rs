// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that values returned by `kani::slice::any_slice` satisfy the
// type invariant

// kani-flags: --default-unwind 4

extern crate kani;
use kani::slice::{any_slice, AnySlice};
use kani::Invariant;

#[kani::proof]
fn check_any_slice_valid() {
    let s: AnySlice<char, 3> = any_slice();
    for i in s.get_slice() {
        assert!(i.is_valid());
    }
}
