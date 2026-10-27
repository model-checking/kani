// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a complete `#[kani::loop_modifies]` clause verifies on `for` loops, whether it is
//! written before or after `#[kani::loop_invariant]`.
//! The clauses only list variables declared before the loop. The loop also writes to variables
//! that the user cannot name in the clause: the index and the pattern bindings that the `for`
//! loop rewrite generates, temporaries, and variables declared in the loop body. These used to
//! fail the assigns check (`Check that x is assignable`, `Check that kani_index_.. is assignable`).

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn invariant_then_modifies() {
    let a: [u8; 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    #[kani::loop_modifies(&s)]
    for x in a {
        s += x as u32;
    }
    assert!(s <= 5 * 255);
}

#[kani::proof]
fn modifies_then_invariant() {
    let a: [u8; 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_modifies(&s)]
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    for x in a {
        s += x as u32;
    }
    assert!(s <= 5 * 255);
}

#[kani::proof]
fn tuple_pattern() {
    let a: [(u8, u8); 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 510)]
    #[kani::loop_modifies(&s)]
    for (i, j) in a {
        s += i as u32 + j as u32;
    }
    assert!(s <= 5 * 510);
}

#[kani::proof]
fn ref_pattern() {
    let a: [u8; 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    #[kani::loop_modifies(&s)]
    for &x in a.iter() {
        s += x as u32;
    }
    assert!(s <= 5 * 255);
}

#[kani::proof]
fn wildcard_pattern() {
    let a: [u8; 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_modifies(&s)]
    #[kani::loop_invariant(s == kani::index as u32)]
    for _ in a {
        s += 1;
    }
    assert!(s == 5);
}

#[kani::proof]
fn enumerate_pattern() {
    let a: [u8; 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    #[kani::loop_modifies(&s)]
    for (_k, x) in a.iter().enumerate() {
        s += *x as u32;
    }
    assert!(s <= 5 * 255);
}

#[kani::proof]
fn body_local() {
    let a: [u8; 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 510)]
    #[kani::loop_modifies(&s)]
    for x in a {
        let y = x as u32 * 2;
        s += y;
    }
    assert!(s <= 5 * 510);
}

#[kani::proof]
fn with_prev() {
    let a: [u8; 5] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 255 && prev(s) <= s)]
    #[kani::loop_modifies(&s)]
    for x in a {
        s += x as u32;
    }
    assert!(s <= 5 * 255);
}

/// The bounds check of `out[0]` needs a basic block of its own. When the clause is written after
/// the invariant, it is evaluated between the first pattern assignment and the loop head.
#[kani::proof]
fn indexed_place_invariant_then_modifies() {
    let a: [u8; 5] = kani::any();
    let mut out: [u32; 2] = [0; 2];
    #[kani::loop_invariant(out[0] <= kani::index as u32 * 255)]
    #[kani::loop_modifies(&out[0])]
    for x in a {
        out[0] += x as u32;
    }
    assert!(out[0] <= 5 * 255);
}

#[kani::proof]
fn indexed_place_modifies_then_invariant() {
    let a: [u8; 5] = kani::any();
    let mut out: [u32; 2] = [0; 2];
    #[kani::loop_modifies(&out[0])]
    #[kani::loop_invariant(out[0] <= kani::index as u32 * 255)]
    for x in a {
        out[0] += x as u32;
    }
    assert!(out[0] <= 5 * 255);
}

/// Each clause is checked against the writes of its own loop (see `nested_outer_writes_unlisted`
/// in `for_loop_modifies_fail.rs`, whose outer loop writes a variable that its clause does not
/// list).
#[kani::proof]
fn nested_for_loops() {
    let a: [u8; 3] = kani::any();
    let b: [u8; 3] = kani::any();
    let mut s: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 3 * 255)]
    #[kani::loop_modifies(&s)]
    for _x in a {
        #[kani::loop_invariant(s <= on_entry(s) + kani::index as u32 * 255)]
        #[kani::loop_modifies(&s)]
        for y in b {
            s += y as u32;
        }
    }
    assert!(s <= 3 * 3 * 255);
}
