// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! FIXME: CBMC silently ignores struct field projections in decreases clauses.
//! The `#spec_decreases` irep is emitted but goto-instrument does not process
//! complex types — only simple integer variables/expressions are supported.
//! Tracked in: https://github.com/model-checking/kani/issues/3168

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

struct Counter {
    val: u8,
}

#[kani::proof]
fn fixme_decreases_struct_field_harness() {
    let mut c = Counter { val: kani::any_where(|i| *i >= 1 && *i <= 20) };

    #[kani::loop_invariant(c.val >= 1)]
    #[kani::loop_decreases(c.val)]
    while c.val > 1 {
        c.val -= 1;
    }

    assert!(c.val == 1);
}
