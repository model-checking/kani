// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: -Zmir-opt-level=2

// kani-flags: -Z loop-contracts

//! `#[kani::loop_modifies]` must stay enforced under MIR optimization: a write outside the
//! declared set is rejected, and a complete set still verifies.
//! Regression test for https://github.com/model-checking/kani/issues/5053.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn incomplete_modifies_fails() {
    let mut i: u32 = 0;
    let mut hidden: u32 = 0;
    #[kani::loop_invariant(i <= 5)]
    #[kani::loop_modifies(&i)]
    while i < 5 {
        hidden = i; // not in the modifies set
        i = i + 1;
    }
    assert!(i == 5);
    std::hint::black_box(hidden);
}

#[kani::proof]
fn complete_modifies_passes() {
    let mut i: u32 = 0;
    let mut tracked: u32 = 0;
    #[kani::loop_invariant(i <= 5)]
    #[kani::loop_modifies(&i, &tracked)]
    while i < 5 {
        tracked = i; // in the modifies set
        i = i + 1;
    }
    assert!(i == 5);
    std::hint::black_box(tracked);
}
