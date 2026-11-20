// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a `#[kani::loop_modifies]` clause without targets is applied: an empty clause, or a
//! clause whose targets all have zero size, allows no write to a variable declared before the
//! loop. Such a clause used to be ignored, so the write set of the loop was inferred and these
//! harnesses verified.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[derive(Clone, Copy)]
struct Zst;

#[kani::proof]
fn empty_clause() {
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies()]
    while i < 3 {
        i += 1;
    }
}

#[kani::proof]
fn zero_sized_target() {
    let z = Zst;
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&z)]
    while i < 3 {
        i += 1;
    }
    let _ = z;
}

#[kani::proof]
fn zero_sized_targets() {
    let z1 = Zst;
    let z2 = ();
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&z1, &z2)]
    while i < 3 {
        i += 1;
    }
    let _ = (z1, z2);
}

#[kani::proof]
fn for_loop_empty_clause() {
    let a: [u8; 3] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies()]
    for x in a {
        s += x as u32;
    }
    let _ = s;
}
