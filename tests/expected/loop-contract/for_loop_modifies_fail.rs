// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that `#[kani::loop_modifies]` is applied to a `for` loop whether it is written before or
//! after `#[kani::loop_invariant]`, and that it rejects writes outside of the clause.
//! The loops of the first two harnesses write `j`, which is not in the modifies clause, so the
//! assigns check for `j` must fail in both. The `for` loop rewrite used to drop the attributes
//! that follow `#[kani::loop_invariant]`, so `invariant_then_modifies` verified successfully
//! with an inferred assigns clause.
//! The other harnesses check that the variables that Kani adds to the clause (the loop index,
//! the pattern bindings and the variables declared in the loop body) do not allow other writes.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn invariant_then_modifies() {
    let a: [u8; 5] = kani::any();
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i == j)]
    #[kani::loop_modifies(&i)]
    for _x in a {
        i = i.wrapping_add(1);
        j = j.wrapping_add(1);
    }
}

#[kani::proof]
fn modifies_then_invariant() {
    let a: [u8; 5] = kani::any();
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_modifies(&i)]
    #[kani::loop_invariant(i == j)]
    for _x in a {
        i = i.wrapping_add(1);
        j = j.wrapping_add(1);
    }
}

/// The loop writes `j` through a reference declared in the loop body. The reference itself only
/// exists in the loop, so writing it is allowed, but writing `j` through it must still fail.
#[kani::proof]
fn write_through_loop_local() {
    let a: [u8; 5] = kani::any();
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i == j)]
    #[kani::loop_modifies(&i)]
    for _x in a {
        i = i.wrapping_add(1);
        let p = &mut j;
        *p = p.wrapping_add(1);
    }
}

/// The clause only allows writing `out[0]`, so writing `out[1]` must fail.
#[kani::proof]
fn indexed_place_other_element() {
    let a: [u8; 5] = kani::any();
    let mut out: [u32; 2] = [0; 2];
    #[kani::loop_invariant(out[0] <= kani::index as u32 * 255)]
    #[kani::loop_modifies(&out[0])]
    for x in a {
        out[0] += x as u32;
        out[1] = x as u32;
    }
}

/// The outer loop writes `y`, which is not in its clause, while the inner loop has a clause of its
/// own. The clause of the outer loop used to be replaced by the clause of the inner loop, so the
/// outer loop got an inferred write set and this verified.
#[kani::proof]
fn nested_outer_writes_unlisted() {
    let a: [u8; 3] = kani::any();
    let b: [u8; 3] = kani::any();
    let mut s: u32 = 0;
    let mut y: u8 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 3 * 255)]
    #[kani::loop_modifies(&s)]
    for _x in a {
        #[kani::loop_invariant(s <= on_entry(s) + kani::index as u32 * 255)]
        #[kani::loop_modifies(&s)]
        for z in b {
            s += z as u32;
        }
        y = y.wrapping_add(1);
    }
}

/// The same, with the clause of the outer loop written before its invariant.
#[kani::proof]
fn nested_outer_writes_unlisted_clause_first() {
    let a: [u8; 3] = kani::any();
    let b: [u8; 3] = kani::any();
    let mut s: u32 = 0;
    let mut y: u8 = 0;
    #[kani::loop_modifies(&s)]
    #[kani::loop_invariant(s <= kani::index as u32 * 3 * 255)]
    for _x in a {
        #[kani::loop_invariant(s <= on_entry(s) + kani::index as u32 * 255)]
        #[kani::loop_modifies(&s)]
        for z in b {
            s += z as u32;
        }
        y = y.wrapping_add(1);
    }
}
