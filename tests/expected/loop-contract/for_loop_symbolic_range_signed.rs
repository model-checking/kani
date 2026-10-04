// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Signed companion to `for_loop_symbolic_range.rs`: a `for` loop over a
//! symbolic `Range<i64>` whose window crosses zero (and may be empty) must
//! count correctly, exercising the `abs_diff` path of the `Range` model's
//! `len()` on a signed instantiation.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn for_loop_symbolic_range_signed() {
    // `start <= end` is NOT assumed: the range may be empty (start > end).
    let start: i64 = kani::any_where(|t| *t >= -8 && *t <= 8);
    let end: i64 = kani::any_where(|t| *t >= -8 && *t <= 8);

    let mut count: usize = 0;
    #[kani::loop_invariant(count == kani::index)]
    for _i in start..end {
        count += 1;
    }
}
