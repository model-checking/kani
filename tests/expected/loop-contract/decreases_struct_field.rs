// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a decreases clause whose measure is a struct field is checked.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

struct Counter {
    val: u8,
    steps: u8,
}

#[kani::proof]
fn decreases_struct_field_harness() {
    let mut c = Counter { val: kani::any_where(|i| *i >= 1 && *i <= 20), steps: 0 };

    #[kani::loop_invariant(c.val >= 1)]
    #[kani::loop_decreases(c.val)]
    while c.val > 1 {
        c.val -= 1;
        c.steps = c.steps.wrapping_add(1);
    }

    assert!(c.val == 1);
}

#[kani::proof]
fn wrong_decreases_struct_field_harness() {
    let mut c = Counter { val: kani::any_where(|i| *i >= 1 && *i <= 20), steps: 0 };

    #[kani::loop_invariant(c.val >= 1)]
    // Bug: `c.steps` increases.
    #[kani::loop_decreases(c.steps)]
    while c.val > 1 {
        c.val -= 1;
        c.steps = c.steps.wrapping_add(1);
    }
}
