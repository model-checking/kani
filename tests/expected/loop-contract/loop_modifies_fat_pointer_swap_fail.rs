// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A user-written `loop_modifies` clause whose targets include two fat pointers of different
//! element sizes. Tuple layout swaps them, so on `main` an assigns target is mis-sized rather
//! than rejected (no crash). Here the loop writes past the clause's `a[0..2]`, so the assigns
//! check must FAIL. This pins the silent case that the fat-pointer-first crash test cannot catch.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn raw_slice_then_ref_slice_fail() {
    let mut a: [u8; 8] = [0; 8];
    let b: [u32; 2] = [0; 2];
    let mut i: usize = 0;
    #[kani::loop_invariant(i <= 8)]
    #[kani::loop_modifies(core::ptr::slice_from_raw_parts(a.as_ptr(), 2), &b[..], &i)]
    while i < 8 {
        a[i] = 1;
        i += 1;
    }
}
