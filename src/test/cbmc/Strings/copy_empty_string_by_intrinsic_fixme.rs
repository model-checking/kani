// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Make sure we can handle explicit copy_nonoverlapping on empty string

// TODO: https://github.com/model-checking/rmc/issues/241
// The copy_nonoverlapping succeeds, but the final copy back to a slice
// fails:
// [...copy_empty_string_by_intrinsic.assertion.2] line 1035 unreachable code: FAILURE
// [...copy_empty_string_by_intrinsic.assertion.1] line 1037 a panicking function std::result::unwrap_failed is invoked: FAILURE
// [...copy_string.assertion.2] line 28 assertion failed: dest_as_str.len() == l: FAILURE

#![feature(rustc_private)]

extern crate libc;

use std::mem::size_of;
use std::ptr::copy_nonoverlapping;
use std::slice::from_raw_parts;
use std::str;

fn copy_string(s: &str, l: usize) {
    unsafe {
        // Unsafe buffer
        let size: libc::size_t = size_of::<u8>();
        let dest: *mut u8 = libc::malloc(size * l) as *mut u8;

        // Copy
        let src = from_raw_parts(s.as_ptr(), l).as_ptr();
        copy_nonoverlapping(src, dest, l);

        // The chunk below causes the 3 failures at the top of the file
        // Back to str, check length
        let dest_slice: &[u8] = from_raw_parts(dest, l);
        let dest_as_str: &str = str::from_utf8(dest_slice).unwrap();
        assert!(dest_as_str.len() == l);
    }
}

fn main() {
    // Verification fails for both of these cases.
    copy_string("x", 1);
    copy_string("", 0);
}
