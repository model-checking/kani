// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that nested loops whose `#[kani::loop_modifies]` clauses list everything that they
//! write verify, with a clause on both loops or on one of them. Each clause is checked against
//! the writes of its own loop. The clause of an outer loop does not need to list the variables
//! declared in its body, such as the counter of an inner loop.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn both_loops_with_clause() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 3 && s == i as u32 * 3)]
    #[kani::loop_modifies(&i, &j, &s)]
    while i < 3 {
        j = 0;
        #[kani::loop_invariant(j <= 3 && s == i as u32 * 3 + j as u32)]
        #[kani::loop_modifies(&j, &s)]
        while j < 3 {
            s += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(s == 9);
}

#[kani::proof]
fn outer_loop_with_clause() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 3 && s == i as u32 * 3)]
    #[kani::loop_modifies(&i, &j, &s)]
    while i < 3 {
        j = 0;
        #[kani::loop_invariant(j <= 3 && s == i as u32 * 3 + j as u32)]
        while j < 3 {
            s += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(s == 9);
}

#[kani::proof]
fn inner_loop_with_clause() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 3 && s == i as u32 * 3)]
    while i < 3 {
        j = 0;
        #[kani::loop_invariant(j <= 3 && s == i as u32 * 3 + j as u32)]
        #[kani::loop_modifies(&j, &s)]
        while j < 3 {
            s += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(s == 9);
}

#[kani::proof]
fn body_local_counter() {
    let mut i: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 3 && s == i as u32 * 3)]
    #[kani::loop_modifies(&i, &s)]
    while i < 3 {
        let mut j: u8 = 0;
        #[kani::loop_invariant(j <= 3 && s == i as u32 * 3 + j as u32)]
        #[kani::loop_modifies(&j, &s)]
        while j < 3 {
            s += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(s == 9);
}

#[kani::proof]
fn labeled_continue() {
    let mut i: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 3 && s == i as u32)]
    #[kani::loop_modifies(&i, &s)]
    'outer: while i < 3 {
        i += 1;
        let mut j: u8 = 0;
        #[kani::loop_invariant(j <= 1)]
        #[kani::loop_modifies(&j, &s)]
        while j < 3 {
            j += 1;
            s += 1;
            continue 'outer;
        }
    }
    assert!(s == 3);
}
