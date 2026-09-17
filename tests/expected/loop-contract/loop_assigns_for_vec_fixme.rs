// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check the use of loop_modifies for Rust's vec
//!
//! FIXME: known failing as of nightly-2026-06-01. `core::ptr::drop_in_place` is no longer the drop
//! lang item; it is now a wrapper that calls the new `drop_glue` lang item through `&mut *to_drop`.
//! Creating that reference asserts the pointee is aligned and valid, so dropping `v` after the loop
//! now requires the loop contract to establish those facts. `loop_modifies` havocs the word holding
//! `v.len()` and the invariant only constrains `i`, so the drop sees an unconstrained length and the
//! reference creation fails. Strengthening the invariant with `v.len() == i * 3 + 3` is rejected
//! ("Rust intrinsic assumption failed"), so expressing what the drop needs is a loop-contracts
//! limitation rather than something the toolchain upgrade can fix. Tracked in
//! <https://github.com/model-checking/kani/issues/4761>.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

use std::ptr;
use std::ptr::slice_from_raw_parts;

#[kani::proof]
#[kani::solver(z3)]
fn main() {
    let mut i = 0;
    let a: [u8; 3] = kani::any();
    let mut v: Vec<u8> = Vec::with_capacity(12);
    v.extend(a);
    //unsafe {(&v as *const Vec<u8>  as *const usize).add(2)} is the ptr to v.len
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&i, slice_from_raw_parts(v.as_ptr(), 12), unsafe {(&v as *const Vec<u8>  as *const usize).add(2)})]
    while i < 3 {
        unsafe {
            ptr::copy_nonoverlapping(v.as_ptr(), (v.as_mut_ptr()).add(i * 3 + 3), 3);
            i = i + 1;
            v.set_len(i * 3 + 3)
        }
    }
}
