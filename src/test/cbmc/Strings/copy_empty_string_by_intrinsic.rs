// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Make sure we can handle explicit copy_nonoverlapping on empty string

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
    }
}

fn main() {
    copy_string("x", 1);
    copy_string("", 0);
}
