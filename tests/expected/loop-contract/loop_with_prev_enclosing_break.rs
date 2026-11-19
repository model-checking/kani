// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A `break` (or `continue`) from the body of a loop whose invariant uses `prev` to a loop
//! enclosing it cannot be expressed in the closure that runs the first iteration, so it is
//! rejected. It used to be rewritten as if it ended only the inner loop: the code after the
//! inner loop then ran although the `break 'outer` skips it, and the assertion below, which fails
//! natively, verified.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn break_enclosing_loop() {
    let mut after_inner = false;
    let mut n: u8 = 0;
    'outer: while n < 1 {
        n += 1;
        let mut k: u8 = 2;
        #[kani::loop_invariant(k <= 2 && k < prev(k))]
        while k > 0 {
            k -= 1;
            if k == 1 {
                break 'outer;
            }
        }
        after_inner = true;
    }
    assert!(after_inner);
}
