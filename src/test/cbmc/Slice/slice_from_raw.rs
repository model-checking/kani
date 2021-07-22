// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

use std::slice;

include!("../../rmc-prelude.rs");

// From Listing 19-7: Creating a slice from an arbitrary memory location. https://doc.rust-lang.org/book/ch19-01-unsafe-rust.html
fn main() {
    let address = 0x01234usize;
    let r = address as *mut i32;
    let slice: &mut [i32] = unsafe { slice::from_raw_parts_mut(r, 10000) };
    // the behavior is undefined when the slice is used
    slice[9999] = 0; // verification succeeds
    assert!(slice[9999] == 0); // verification succeeds
}
