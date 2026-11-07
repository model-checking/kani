// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that the decreases clauses of nested loops are checked on their own loop.
//! The inner loop is loop 0 and the outer loop is loop 1.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn nested_both_harness() {
    let mut i: u8 = 10;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 10)]
    #[kani::loop_decreases(i)]
    while i > 0 {
        j = 5;
        #[kani::loop_invariant(j <= 5)]
        #[kani::loop_decreases(j)]
        while j > 0 {
            j -= 1;
        }
        i -= 1;
    }
}

#[kani::proof]
fn nested_outer_harness() {
    let mut i: u8 = 10;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 10)]
    #[kani::loop_decreases(i)]
    while i > 0 {
        j = 5;
        #[kani::loop_invariant(j <= 5)]
        while j > 0 {
            j -= 1;
        }
        i -= 1;
    }
}

#[kani::proof]
fn nested_inner_harness() {
    let mut i: u8 = 10;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 10)]
    while i > 0 {
        j = 5;
        #[kani::loop_invariant(j <= 5)]
        #[kani::loop_decreases(j)]
        while j > 0 {
            j -= 1;
        }
        i -= 1;
    }
}

#[kani::proof]
fn nested_wrong_outer_harness() {
    let mut i: u8 = 10;
    let mut up: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 10 && up == 10 - i)]
    // Bug: `up` increases.
    #[kani::loop_decreases(up)]
    while i > 0 {
        j = 5;
        #[kani::loop_invariant(j <= 5)]
        #[kani::loop_decreases(j)]
        while j > 0 {
            j -= 1;
        }
        i -= 1;
        up += 1;
    }
}

#[kani::proof]
fn nested_wrong_inner_harness() {
    let mut i: u8 = 10;
    let mut j: u8 = 0;
    let mut up: u8 = 0;
    #[kani::loop_decreases(i)]
    #[kani::loop_invariant(i <= 10)]
    while i > 0 {
        j = 5;
        up = 0;
        // Bug: `up` increases.
        #[kani::loop_decreases(up)]
        #[kani::loop_invariant(j <= 5 && up == 5 - j)]
        while j > 0 {
            j -= 1;
            up += 1;
        }
        i -= 1;
    }
}
