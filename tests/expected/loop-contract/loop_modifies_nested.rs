// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that nested loops whose `#[kani::loop_modifies]` clauses list everything that they
//! write verify, with a clause on both loops or on one of them, and with three levels of loops.
//! Each clause is checked against the writes of its own loop. The clause of an outer loop does
//! not need to list the variables declared in its body, such as the counter of an inner loop.

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

/// The innermost clause has a single target, and the middle loop declares a variable that it
/// writes after the innermost loop. The binding of the innermost clause must not be added to the
/// write set of the middle loop, which would fail `Check assigns clause inclusion` for the middle
/// loop.
#[kani::proof]
fn three_levels() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut k: u8 = 0;
    let mut z: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i, &j, &k, &z)]
    while i < 2 {
        j = 0;
        #[kani::loop_invariant(j <= 2)]
        #[kani::loop_modifies(&j, &k, &z)]
        while j < 2 {
            let t: u8 = j;
            k = 0;
            #[kani::loop_invariant(k <= 2)]
            #[kani::loop_modifies(&k)]
            while k < 2 {
                k += 1;
            }
            z = t;
            j += 1;
        }
        i += 1;
    }
}

/// The innermost loop has an empty clause and does not iterate.
#[kani::proof]
fn three_levels_empty_innermost() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i, &j)]
    while i < 2 {
        j = 0;
        #[kani::loop_invariant(j <= 2)]
        #[kani::loop_modifies(&j)]
        while j < 2 {
            #[kani::loop_invariant(true)]
            #[kani::loop_modifies()]
            loop {
                break;
            }
            j += 1;
        }
        i += 1;
    }
}
