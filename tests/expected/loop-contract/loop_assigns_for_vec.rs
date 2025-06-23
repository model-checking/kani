// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check the use of loop_assigns for Rust's vec

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

use std::ptr;
use std::ptr::slice_from_raw_parts;

#[kani::proof]
fn main() {
    let mut i = 0;
    let a: [u8; 3] = kani::any();
    let mut v: Vec<u8> = Vec::with_capacity(12);
    v.extend(a);
    //unsafe {(&v as *const Vec<u8>  as *const usize).add(2)} is the ptr to v.len
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_assigns(&i, slice_from_raw_parts(v.as_ptr(), 12), unsafe {(&v as *const Vec<u8>  as *const usize).add(2)})]
    while i < 3 {
        unsafe {
            ptr::copy_nonoverlapping(v.as_ptr(), (v.as_mut_ptr()).add(i * 3 + 3), 3);
            i = i + 1;
            v.set_len(i * 3 + 3)
        }
    }
}
