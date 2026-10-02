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
