// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that the clause of a loop with several `#[kani::loop_modifies]` attributes has the
//! targets of all of them, whatever the order of the attributes. Only the attribute closest to
//! the loop used to be applied, so the loops that verify here failed the assigns check of the
//! targets of the other attributes.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn invariant_first() {
    let mut i: u8 = 0;
    let mut w: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i)]
    #[kani::loop_modifies(&w)]
    while i < 2 {
        w = i;
        i += 1;
    }
    let _ = w;
}

#[kani::proof]
fn modifies_first() {
    let mut i: u8 = 0;
    let mut w: u8 = 0;
    #[kani::loop_modifies(&i)]
    #[kani::loop_modifies(&w)]
    #[kani::loop_invariant(i <= 2)]
    while i < 2 {
        w = i;
        i += 1;
    }
    let _ = w;
}

#[kani::proof]
fn modifies_around_invariant() {
    let mut i: u8 = 0;
    let mut w: u8 = 0;
    #[kani::loop_modifies(&i)]
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&w)]
    while i < 2 {
        w = i;
        i += 1;
    }
    let _ = w;
}

#[kani::proof]
fn for_loop() {
    let a: [u8; 3] = kani::any();
    let mut s: u32 = 0;
    let mut w: u8 = 0;
    #[kani::loop_modifies(&s)]
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    #[kani::loop_modifies(&w)]
    for x in a {
        s += x as u32;
        w = x;
    }
    assert!(s <= 3 * 255);
    let _ = w;
}

/// An empty clause adds no target, so the write to `w` must fail its assigns check.
#[kani::proof]
fn with_empty_clause_fail() {
    let mut i: u8 = 0;
    let mut w: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i)]
    #[kani::loop_modifies()]
    while i < 2 {
        w = i;
        i += 1;
    }
    let _ = w;
}
