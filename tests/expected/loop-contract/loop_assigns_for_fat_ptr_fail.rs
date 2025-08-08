// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check the use of loop_modifies for Rust's Fat ptr

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

use std::ptr::slice_from_raw_parts;

#[kani::proof]
fn main() {
    let mut i = 0;
    let mut a: [u8; 100] = kani::any();
    #[kani::loop_invariant(i <= 20)]
    #[kani::loop_modifies(&i , slice_from_raw_parts(a.as_ptr(), 18))]
    while i < 20 {
        a[i] = 1;
        i = i + 1;
    }
}
