// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a decreases clause whose measure is a variable or a place (here, the target of a
//! reference) is checked at each iteration.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn variable_measure_harness() {
    let mut i: usize = 0;
    let mut remaining: u16 = 4;

    #[kani::loop_invariant(i <= 4 && remaining as usize + i == 4)]
    #[kani::loop_decreases(remaining)]
    while i < 4 {
        remaining = remaining - 1;
        i += 1;
    }
}

#[kani::proof]
fn variable_measure_for_loop_harness() {
    let a: [u8; 5] = kani::any();
    let mut remaining: u16 = 5;

    #[kani::loop_decreases(remaining)]
    #[kani::loop_invariant(remaining as usize + kani::index == 5)]
    for _x in a {
        remaining = remaining - 1;
    }
}

#[kani::proof]
fn variable_measure_with_prev_harness() {
    let mut x: u8 = kani::any_where(|v| *v >= 2 && *v <= 50);

    #[kani::loop_invariant(x >= 1 && x <= 50 && prev(x) == x + 1)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        x = x - 1;
    }
}

fn count_down(n: &mut u32) {
    #[kani::loop_invariant(true)]
    #[kani::loop_decreases(*n)]
    while *n > 0 {
        *n -= 1;
    }
}

#[kani::proof]
fn deref_measure_harness() {
    let mut n: u32 = kani::any();
    count_down(&mut n);
    assert!(n == 0);
}
