// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that `#[kani::loop_modifies]` is applied to a `while let` loop whether it is written
//! before or after `#[kani::loop_invariant]`.
//! Both loops write `j`, which is not in the modifies clause, so the assigns check for `j` must
//! fail in both harnesses. The `while let` rewrite used to drop the attributes that follow
//! `#[kani::loop_invariant]`, so `invariant_then_modifies` verified successfully with an
//! inferred assigns clause.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn invariant_then_modifies() {
    let mut k: u8 = kani::any();
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i == j)]
    #[kani::loop_modifies(&k, &i)]
    while let Some(n) = k.checked_sub(1) {
        k = n;
        i = i.wrapping_add(1);
        j = j.wrapping_add(1);
    }
}

#[kani::proof]
fn modifies_then_invariant() {
    let mut k: u8 = kani::any();
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_modifies(&k, &i)]
    #[kani::loop_invariant(i == j)]
    while let Some(n) = k.checked_sub(1) {
        k = n;
        i = i.wrapping_add(1);
        j = j.wrapping_add(1);
    }
}
