// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! An outer loop with an explicit `#[kani::loop_modifies]` writes the locals that Kani generates for
//! the loops nested in it, which the user can't name. Kani adds them to the outer clause, so these
//! harnesses verify with a clause that lists only the user's variables.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

/// The inner loop is a `for` loop over a range, which gets a generated index and length.
#[kani::proof]
fn inner_for_over_range() {
    let mut i: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&i, &s)]
    while i < 3 {
        #[kani::loop_invariant(true)]
        for _ in 0..4u8 {
            s = s.wrapping_add(1);
        }
        i += 1;
    }
}

/// The inner loop is a `for` loop over an array.
#[kani::proof]
fn inner_for_over_array() {
    let a: [u8; 3] = [1, 2, 3];
    let mut i: u8 = 0;
    let mut s: u32 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i, &s)]
    while i < 2 {
        #[kani::loop_invariant(true)]
        for x in a {
            s = s.wrapping_add(x as u32);
        }
        i += 1;
    }
}

/// The inner loop has an invariant but no clause of its own.
#[kani::proof]
fn inner_invariant_without_clause() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_modifies(&i, &j)]
    while i < 3 {
        j = 0;
        #[kani::loop_invariant(j <= 3)]
        while j < 3 {
            j += 1;
        }
        i += 1;
    }
}

/// Three levels of nesting, with a clause only on the outermost loop.
#[kani::proof]
fn three_levels() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut k: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i, &j, &k)]
    while i < 2 {
        j = 0;
        #[kani::loop_invariant(j <= 2)]
        while j < 2 {
            k = 0;
            #[kani::loop_invariant(k <= 2)]
            while k < 2 {
                k += 1;
            }
            j += 1;
        }
        i += 1;
    }
}
