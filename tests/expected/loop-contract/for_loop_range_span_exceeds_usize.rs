// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A 128-bit `Range` can span more elements than `usize::MAX`. The `Range`
//! model's `len()` must reject such spans instead of truncating: an `as usize`
//! truncation would model a non-empty range as EMPTY and verify the loop
//! vacuously. All harnesses must FAIL on the model's span check.

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

// A span of exactly 2^64 makes the truncation visible directly: `as usize` casts it to 0, so on
// `main` the body is skipped and the loop verifies vacuously (`assert!(false)` never runs). The
// span check must reject it, so the body becomes reachable and this harness FAILS.
#[kani::proof]
fn unsigned_span_truncates_to_zero() {
    #[kani::loop_invariant(true)]
    for _i in 0u128..(1u128 << 64) {
        assert!(false);
    }
}
