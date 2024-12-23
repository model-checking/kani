// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z mem-predicates
//! Check that Kani detects UB for offset_from for pointers to ZSTs.
//! The pointer::offset_from method has an assertion to check for ZSTs *before* it calls the intrinsic ptr_offset_from.
//! Since we model the instrinsic, our ZST assertion comes too late, after the rustc assertion has actually failed.
//! So for now, this test does not actually test our modeling of the intrinsic, since verification fails before the intrinsic call, because of the failed rustc assertion.
//! But the Rust developers may remove the assertion in the future, in which case this test would prevent regression.
extern crate kani;

#[kani::proof]
fn check_offset_from_zst_ub() {
    let x = ();
    let ptr_1 = &x as *const ();
    let ptr_2 = &x as *const ();
    unsafe { ptr_1.offset_from(ptr_2) };
}
