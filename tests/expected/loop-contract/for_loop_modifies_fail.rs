// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that `#[kani::loop_modifies]` is applied to a `for` loop whether it is written before or
//! after `#[kani::loop_invariant]`.
//! Both loops write `j`, which is not in the modifies clause, so the assigns check for `j` must
//! fail in both harnesses. The `for` loop rewrite used to drop the attributes that follow
//! `#[kani::loop_invariant]`, so `invariant_then_modifies` verified successfully with an
//! inferred assigns clause.
//! Other assigns checks (for locals introduced by the `for` loop rewrite) may fail as well; this
//! test only checks the one for `j`.

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
