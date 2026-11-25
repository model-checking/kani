// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A variable that the loop body declares and that a `return` in the loop body reads does not
//! need to be in an explicit `#[kani::loop_modifies]` clause: it is dead at the loop head.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn first_above(n: u8) -> u8 {
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 10)]
    #[kani::loop_modifies(&i)]
    while i < 10 {
        let found = i + 1;
        if found > n {
            return found;
        }
        i += 1;
    }
    0
}

#[kani::proof]
fn main() {
    let n: u8 = kani::any_where(|n| *n < 10);
    assert!(first_above(n) <= 10);
}
