// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check for-loop invariant for StepBy.
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    #[kani::loop_invariant( kaniindex <= 5 && sum <= (kaniindex as u32 * 14 as u32) )]
    for i in (1..15).step_by(3) {
        sum = sum + (i as u32);
    }
    assert!(sum <= 70);
}
