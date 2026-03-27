// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check decreases clause on a `loop` (not while).
//! Inspired by Verus loop_decreases1 test.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn loop_loop_decreases_harness() {
    let mut i: u8 = 100;

    #[kani::loop_invariant(i <= 100 && i >= 2 && i % 2 == 0)]
    #[kani::loop_decreases(i)]
    loop {
        i = i - 2;
        if i == 2 {
            break;
        }
    }

    assert!(i == 2);
}
