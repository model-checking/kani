// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that the clause of a loop with several `#[kani::loop_modifies]` attributes has the
//! targets of all of them, whatever the order of the attributes. Only the attribute closest to
//! the loop used to be applied, so the `while` loops that verify here failed the assigns check of
//! the targets of the other attributes. Also check targets whose order in the layout of the tuple
//! that holds them is not their order in the clause, which Kani used to mix up.

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

/// The first target is a slice, so the targets are in another order in the layout of the tuple
/// that holds them.
#[kani::proof]
fn slice_first() {
    let mut a: [u8; 4] = [0; 4];
    let mut i: usize = 0;
    let mut j: usize = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(core::ptr::slice_from_raw_parts_mut(a.as_mut_ptr(), 2))]
    #[kani::loop_modifies(&i, &j)]
    while i < 2 {
        a[i] = 1;
        j = i;
        i += 1;
    }
    let _ = j;
}

/// The same with slices of different element sizes: the clause only allows writing `a[0]`, so
/// the write to `a[1]` must fail its assigns check.
#[kani::proof]
fn slice_first_fail() {
    let mut a: [u8; 8] = [0; 8];
    let mut b: [u64; 2] = [0; 2];
    let mut i: usize = 0;
    #[kani::loop_invariant(i <= 1)]
    #[kani::loop_modifies(core::ptr::slice_from_raw_parts_mut(a.as_mut_ptr(), 1), &mut b[..], &i)]
    while i < 1 {
        a[1] = 1;
        i += 1;
    }
    let _ = b;
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
