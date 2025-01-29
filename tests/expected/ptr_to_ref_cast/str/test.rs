// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(set_ptr_value)]

//! This test checks that Kani detects UB resulting from converting a raw
//! pointer to a reference when the metadata is not valid.

#[kani::proof]
fn check_with_metadata_fail() {
    let short = "sh";
    let long = "longer";
    let ptr = short as *const str;
    // This should trigger UB since the slice is not valid for the new length.
    let fake_long = unsafe { &*ptr.with_metadata_of(long) };
    assert_eq!(fake_long.len(), long.len());
}
