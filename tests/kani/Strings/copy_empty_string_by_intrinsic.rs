// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --mir-linker
//! Make sure we can handle explicit copy_nonoverlapping on empty string
//! This used to trigger an issue: https://github.com/model-checking/kani/issues/241

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

#[kani::proof]
fn main() {
    copy_string("x", 1);
    copy_string("", 0);
}
