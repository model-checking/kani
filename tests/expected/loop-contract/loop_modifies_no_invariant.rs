// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that Kani warns about a `#[kani::loop_modifies]` clause on a loop without a loop
//! invariant, and that it does not apply the clause to another loop. A loop without a loop
//! invariant is unwound. The clause used to be applied to the next loop with a loop invariant,
//! so these harnesses failed with `Check that j is assignable`.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
#[kani::unwind(4)]
fn loop_without_invariant() {
    let mut i: u8 = 0;
    #[kani::loop_modifies(&i)]
    while i < 3 {
        i += 1;
    }
    // This loop has no clause, and writes `j`, which is not in the clause above.
    let mut j: u8 = 0;
    #[kani::loop_invariant(j <= 2)]
    while j < 2 {
        j += 1;
    }
    assert!(i == 3);
}

/// The loop without a loop invariant does not iterate.
#[kani::proof]
fn loop_without_invariant_or_back_edge() {
    let mut a: u8 = 0;
    #[kani::loop_modifies(&a)]
    loop {
        a = 1;
        break;
    }
    // This loop has no clause, and writes `j`, which is not in the clause above.
    let mut j: u8 = 0;
    #[kani::loop_invariant(j <= 2)]
    while j < 2 {
        j += 1;
    }
    assert!(a == 1);
}
