// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that the `#[kani::loop_modifies]` clause of an outer loop is applied to the outer loop,
//! not to the inner loop (and, in `sequential_loops`, that the clause of a loop is not taken by
//! the following loop). In each harness, a loop with a clause writes `t`, which is not in its
//! modifies clause, so the assigns check for `t` must fail.
//! The clause of the outer loop used to be attached to the inner loop instead: it was ignored if
//! the inner loop had a clause of its own, and checked against the writes of the inner loop
//! otherwise.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn both_loops_with_clause() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut s: u32 = 0;
    let mut t: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&i, &j, &s)]
    while i < 3 {
        j = 0;
        #[kani::loop_invariant(j <= 3)]
        #[kani::loop_modifies(&j, &s)]
        while j < 3 {
            s = s.wrapping_add(1);
            j += 1;
        }
        t = t.wrapping_add(1);
        i += 1;
    }
}

#[kani::proof]
fn outer_loop_with_clause() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut s: u32 = 0;
    let mut t: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&i, &j, &s)]
    while i < 3 {
        j = 0;
        #[kani::loop_invariant(j <= 3)]
        while j < 3 {
            s = s.wrapping_add(1);
            j += 1;
        }
        t = t.wrapping_add(1);
        i += 1;
    }
}

/// The `on_entry` expression of the outer invariant has a branch, so there is a join between the
/// outer `loop_modifies` binding (written first) and the outer loop head.
#[kani::proof]
fn join_before_outer_loop() {
    let n: u8 = kani::any();
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut s: u32 = 0;
    let mut t: u8 = 0;
    #[kani::loop_modifies(&i, &j, &s)]
    #[kani::loop_invariant(i <= 3 && on_entry(if n > 3 { 3 } else { n }) <= 3)]
    while i < 3 {
        j = 0;
        #[kani::loop_invariant(j <= 3)]
        #[kani::loop_modifies(&j, &s)]
        while j < 3 {
            s = s.wrapping_add(1);
            j += 1;
        }
        t = t.wrapping_add(1);
        i += 1;
    }
}

/// The clause of the `for` loop is written before the invariant, so it is assigned before the
/// `if` of the `for` loop rewrite, which also dominates the following loop.
#[kani::proof]
fn sequential_loops() {
    let a: [u8; 3] = kani::any();
    let mut i: u8 = 0;
    let mut s: u32 = 0;
    let mut t: u8 = 0;
    #[kani::loop_modifies(&s)]
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    for x in a {
        s += x as u32;
        t = t.wrapping_add(1);
    }
    #[kani::loop_invariant(i <= 3)]
    while i < 3 {
        i += 1;
    }
}

/// Three nested loops, whose middle loop writes `t`.
#[kani::proof]
fn three_levels() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut k: u8 = 0;
    let mut t: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i, &j, &k, &t)]
    while i < 2 {
        j = 0;
        #[kani::loop_invariant(j <= 2)]
        #[kani::loop_modifies(&j, &k)]
        while j < 2 {
            k = 0;
            #[kani::loop_invariant(k <= 2)]
            #[kani::loop_modifies(&k)]
            while k < 2 {
                k += 1;
            }
            t = t.wrapping_add(1);
            j += 1;
        }
        i += 1;
    }
}
