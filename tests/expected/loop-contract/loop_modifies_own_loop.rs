// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that the `#[kani::loop_modifies]` clause of a loop is applied to that loop only, in both
//! attribute orders:
//! - the clause of an outer loop is not applied to an inner loop without a clause, which writes a
//!   variable declared in the body of the outer loop;
//! - the clause of a loop that does not iterate (`loop { break; }`) is not applied to the next
//!   loop.
//!
//! Codegen used to attach a clause to the next loop latch that it generated, so these harnesses
//! failed with `Check that acc is assignable` or `Check that j is assignable`.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn for_outer_clause_after_invariant() {
    let a: [u8; 2] = kani::any();
    let b: [u8; 2] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 2 * 255)]
    #[kani::loop_modifies(&s)]
    for _x in a {
        let mut acc: u32 = 0;
        #[kani::loop_invariant(acc <= kani::index as u32 * 255)]
        for y in b {
            acc += y as u32;
        }
        s += acc;
    }
    assert!(s <= 2 * 2 * 255);
}

#[kani::proof]
fn for_outer_clause_before_invariant() {
    let a: [u8; 2] = kani::any();
    let b: [u8; 2] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_modifies(&s)]
    #[kani::loop_invariant(s <= kani::index as u32 * 2 * 255)]
    for _x in a {
        let mut acc: u32 = 0;
        #[kani::loop_invariant(acc <= kani::index as u32 * 255)]
        for y in b {
            acc += y as u32;
        }
        s += acc;
    }
    assert!(s <= 2 * 2 * 255);
}

#[kani::proof]
fn while_outer_clause_after_invariant() {
    let mut i: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 2 && s <= i as u32 * 2 * 255)]
    #[kani::loop_modifies(&i, &s)]
    while i < 2 {
        let b: [u8; 2] = kani::any();
        let mut acc: u32 = 0;
        let mut j: usize = 0;
        #[kani::loop_invariant(j <= 2 && acc <= j as u32 * 255)]
        while j < 2 {
            acc += b[j] as u32;
            j += 1;
        }
        s += acc;
        i += 1;
    }
    assert!(s <= 2 * 2 * 255);
}

#[kani::proof]
fn while_outer_clause_before_invariant() {
    let mut i: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_modifies(&i, &s)]
    #[kani::loop_invariant(i <= 2 && s <= i as u32 * 2 * 255)]
    while i < 2 {
        let b: [u8; 2] = kani::any();
        let mut acc: u32 = 0;
        let mut j: usize = 0;
        #[kani::loop_invariant(j <= 2 && acc <= j as u32 * 255)]
        while j < 2 {
            acc += b[j] as u32;
            j += 1;
        }
        s += acc;
        i += 1;
    }
    assert!(s <= 2 * 2 * 255);
}

#[kani::proof]
fn empty_clause_of_loop_without_latch() {
    let mut x: u8 = 0;
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies()]
    loop {
        break;
    }
    let mut j: u8 = 0;
    #[kani::loop_invariant(j <= 2)]
    while j < 2 {
        j += 1;
    }
    x = 1;
    assert!(x == 1);
}

#[kani::proof]
fn clause_of_loop_without_latch() {
    let mut x: u8 = 0;
    #[kani::loop_modifies(&x)]
    #[kani::loop_invariant(true)]
    loop {
        break;
    }
    let mut j: u8 = 0;
    #[kani::loop_invariant(j <= 2)]
    while j < 2 {
        j += 1;
    }
    x = 1;
    assert!(x == 1);
}
