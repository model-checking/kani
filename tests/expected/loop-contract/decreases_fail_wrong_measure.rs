// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a decreases clause whose measure is a variable that increases fails the decreases
//! check. The loop invariants hold and nothing overflows, so the decreases check is the only
//! check that fails.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn wrong_decreases_harness() {
    let mut i: usize = 0;
    let mut m: u16 = 0;

    #[kani::loop_invariant(i <= 4 && m as usize == i)]
    #[kani::loop_decreases(m)]
    while i < 4 {
        // Bug: the measure increases instead of decreasing.
        m = m + 1;
        i += 1;
    }
}

#[kani::proof]
fn wrong_decreases_for_loop_harness() {
    let a: [u8; 5] = kani::any();
    let mut m: u16 = 0;

    #[kani::loop_decreases(m)]
    #[kani::loop_invariant(m as usize == kani::index)]
    for _x in a {
        // Bug: the measure increases instead of decreasing.
        m = m + 1;
    }
}

#[kani::proof]
fn wrong_decreases_while_let_harness() {
    let mut n: u8 = kani::any_where(|v| *v <= 10);
    let mut up: u8 = 0;

    // The decreases clause is after the loop invariant.
    #[kani::loop_invariant(n <= 10 && up <= 10 && up + n <= 10)]
    #[kani::loop_decreases(up)]
    while let Some(m) = n.checked_sub(1) {
        n = m;
        // Bug: the measure increases instead of decreasing.
        up += 1;
    }
}

#[kani::proof]
fn wrapping_decreases_harness() {
    let mut i: u8 = 0;
    let mut m: u8 = kani::any();

    #[kani::loop_invariant(i <= 4)]
    #[kani::loop_decreases(m)]
    while i < 4 {
        // Bug: the measure increases when it wraps around from 0.
        m = m.wrapping_sub(1);
        i += 1;
    }
}
