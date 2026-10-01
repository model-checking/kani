// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A `break` of a loop nested in the body of a loop whose invariant uses `prev` ends that nested
//! loop only. It used to be rewritten as if it ended the outer loop, so the outer loop stopped
//! after its first iteration and the assertion below, which fails in the third iteration, was
//! never reached: Kani reported SUCCESS.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn nested_break_does_not_end_outer_loop() {
    let mut k: u8 = 7;
    #[kani::loop_invariant(k <= 7 && k < prev(k))]
    while k > 0 {
        k -= 1;
        let mut m: u8 = 0;
        #[kani::loop_invariant(m <= 2)]
        while m < 2 {
            m += 1;
            if m == 1 {
                break;
            }
        }
        assert!(k != 5);
    }
}
