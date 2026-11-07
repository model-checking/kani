// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that Kani warns about a decreases clause that it cannot attach to a loop with a loop
//! invariant, and that it does not check the clause on another loop. A loop without a loop
//! invariant is unwound, and a loop whose body always exits does not loop.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
#[kani::unwind(12)]
fn decreases_without_invariant_harness() {
    let mut i: u32 = 10;
    #[kani::loop_decreases(i)]
    while i > 0 {
        i -= 1;
    }
    // This loop does not have a decreases clause.
    let mut j: u32 = 0;
    #[kani::loop_invariant(j <= 5)]
    while j < 5 {
        j += 1;
    }
    assert!(i == 0 && j == 5);
}

#[kani::proof]
fn loop_without_back_edge_harness() {
    let mut x: u8 = kani::any_where(|v| *v >= 1 && *v <= 10);
    #[kani::loop_invariant(x <= 10)]
    #[kani::loop_decreases(x)]
    while x > 0 {
        x -= 1;
        break;
    }
    // This loop does not have a decreases clause, and does not modify `x`.
    let mut j: u8 = 0;
    #[kani::loop_invariant(j <= 5)]
    while j < 5 {
        j += 1;
    }
}
