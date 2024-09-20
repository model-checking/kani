// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(set_ptr_value)]

//! This test checks that Kani detects UB resulting from converting a raw
//! pointer to a reference when the metadata is not valid.

// Generate invalid fat pointer by combining the metadata.
#[kani::proof]
fn check_with_metadata_fail() {
    let short = [0u32; 2];
    let long = [0u32; 10];
    let ptr = &short as *const [u32];
    // This should trigger UB since the slice is not valid for the new length.
    let fake_long = unsafe { &*ptr.with_metadata_of(&long) };
    assert_eq!(fake_long.len(), long.len());
}

#[kani::proof]
fn check_with_byte_add_fail() {
    let data = [5u8; 5];
    let ptr = &data as *const [u8];
    // This should trigger UB since the metadata does not get adjusted.
    let val = unsafe { &*ptr.byte_add(1) };
    assert_eq!(val.len(), data.len());
}

#[kani::proof]
fn check_with_byte_add_sub_pass() {
    let data = [5u8; 5];
    let ptr = &data as *const [u8];
    let offset = kani::any_where(|i| *i < 100);
    // This should pass since the resulting metadata is valid
    let val = unsafe { &*ptr.byte_add(offset).byte_sub(offset) };
    assert_eq!(val.len(), data.len());
}
