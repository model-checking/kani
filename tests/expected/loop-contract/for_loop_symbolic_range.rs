// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Regression for the `KaniIter for Range` havoc overflow.
//!
//! A `for` loop over a symbolic `Range<usize>` whose bounds are not constrained
//! to `start <= end` must not make the loop model's `len()` underflow. Before the
//! fix, loop-contract havoc could produce `start > end`, and
//! `<Range<usize> as KaniIter>::len` failed with "attempt to subtract with
//! overflow". The fix makes `len()` total (0 when `start >= end`), removing it.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn for_loop_symbolic_range() {
    let start: usize = kani::any();
    let end: usize = kani::any();
    // Bound the iteration space for tractability. Crucially, `start <= end` is
    // NOT assumed here: the range may be empty (start > end), which is exactly
    // the case that made the model's `len()` underflow before the fix.
    kani::assume(start <= 16);
    kani::assume(end <= 16);

    let mut count: usize = 0;
    #[kani::loop_invariant(count == kani::index)]
    for _i in start..end {
        count += 1;
    }
}
