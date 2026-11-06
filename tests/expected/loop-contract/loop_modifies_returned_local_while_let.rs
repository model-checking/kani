// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// compile-flags: --edition 2024
// kani-flags: -Z loop-contracts

//! As `loop_modifies_returned_local.rs`, for the loops that `#[kani::loop_invariant]` rewrites
//! before it names them: `while let` and a let chain. A variable that the loop body declares and
//! that a `return` reads still does not need to be in the clause.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn step(i: u8) -> Option<u8> {
    if i < 10 { Some(i + 1) } else { None }
}

fn first_above_while_let(n: u8) -> u8 {
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 10)]
    #[kani::loop_modifies(&i)]
    while let Some(j) = step(i) {
        let found = j;
        if found > n {
            return found;
        }
        i = j;
    }
    0
}

fn first_above_let_chain(n: u8) -> u8 {
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 10)]
    #[kani::loop_modifies(&i)]
    while i < 10
        && let Some(j) = step(i)
    {
        let found = j;
        if found > n {
            return found;
        }
        i = j;
    }
    0
}

#[kani::proof]
fn check_while_let() {
    let n: u8 = kani::any_where(|n| *n < 10);
    assert!(first_above_while_let(n) <= 10);
}

#[kani::proof]
fn check_let_chain() {
    let n: u8 = kani::any_where(|n| *n < 10);
    assert!(first_above_let_chain(n) <= 10);
}
