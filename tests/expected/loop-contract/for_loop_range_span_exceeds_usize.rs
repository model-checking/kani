// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A 128-bit `Range` can span more elements than `usize::MAX`. The `Range`
//! model's `len()` must reject such spans instead of truncating: an `as usize`
//! truncation would model `i128::MIN..0` (2^127 elements) as EMPTY and verify
//! the loop vacuously. Both harnesses must FAIL on the model's span check.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn signed_wide_span_rejected() {
    let mut last: i128 = 0;
    #[kani::loop_invariant(true)]
    for i in i128::MIN..0 {
        last = i;
    }
    assert!(last <= 0);
}

#[kani::proof]
fn unsigned_wide_span_rejected() {
    let mut last: u128 = 0;
    #[kani::loop_invariant(true)]
    for i in 0..(u128::MAX / 2) {
        last = i;
    }
    assert!(last < u128::MAX);
}
