// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that an empty slice can be a `loop_modifies` target.
//! The data pointer of an empty slice is often dangling (e.g., the one of an empty `Vec` or of a
//! `String` without capacity). CBMC used to reject such a target with
//! `ptr NULL or writable up to size`, even though it covers zero bytes.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

use std::ptr::slice_from_raw_parts_mut;

/// Zero the elements of a vector whose length may be 0.
#[kani::proof]
fn vec_of_any_len() {
    let len: usize = kani::any_where(|l: &usize| *l <= 2);
    let mut v: Vec<u8> = vec![1; len];
    let s = v.as_mut_slice();
    let n = len;
    let mut i: usize = 0;
    #[kani::loop_invariant(i <= n)]
    #[kani::loop_modifies(&i, &mut s[..])]
    while i < n {
        s[i] = 0;
        i += 1;
    }
}

/// The spare capacity of a `String` without capacity.
#[kani::proof]
fn empty_string_buffer() {
    let mut st = String::new();
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&i, slice_from_raw_parts_mut(st.as_mut_ptr(), st.capacity()))]
    while i < 3 {
        i += 1;
    }
}

/// A slice of an empty array.
#[kani::proof]
fn empty_array() {
    let mut a: [u8; 0] = [];
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&i, &mut a[..])]
    while i < 3 {
        i += 1;
    }
}
